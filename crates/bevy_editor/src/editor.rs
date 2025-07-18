//! # Bevy Editor - Core Module
//!
//! This module provides the main editor interface with a comprehensive inspector 
//! for Bevy applications. The editor features a modern, two-panel layout with 
//! real-time entity and component inspection capabilities.
//!
//! ## Features
//!
//! - **Two-Panel Layout**: Entity tree (left) + Component inspector (right)
//! - **Real-time Updates**: Live connection status with visual indicators
//! - **Interactive Selection**: Entity selection with hover effects and feedback
//! - **Modern UI**: Dark theme with professional styling and consistent spacing
//! - **Event-Driven**: Uses Bevy's observer system for clean architecture
//! - **Remote Ready**: Designed for bevy_remote integration
//!
//! ## Architecture
//!
//! The editor uses a modular, event-driven architecture:
//! - `EditorPlugin`: Main orchestrator that combines all components
//! - Panel plugins: Handle specific UI areas (entity list, component inspector)
//! - Widget plugins: Provide reusable UI components
//! - Remote client: Manages communication with bevy_remote servers
//!
//! ## Usage
//!
//! Add the main plugin to your app:
//! ```rust,no_run
//! use bevy::prelude::*;
//! use bevy_editor::prelude::EditorPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(EditorPlugin)
//!     .run();
//! ```

use bevy::prelude::*;
use serde_json::Value;

// Import our modular components
use crate::remote::types::{EditorState, ComponentDisplayState, ComponentDataFetched, RemoteConnection, EntitiesFetched, ConnectionStatus, RemoteEntity, ComponentField};
use crate::panels::{EntityListPlugin, ComponentInspectorPlugin, parse_component_fields};
use crate::widgets::{WidgetsPlugin, ScrollViewBuilder, ScrollContent};
use crate::formatting::{format_value_inline, format_simple_value, is_simple_value, all_numbers};

/// Main plugin for the Bevy Editor that provides a comprehensive inspector interface.
/// 
/// This plugin orchestrates all editor functionality by combining:
/// - Entity list panel for browsing world entities
/// - Component inspector for detailed component viewing
/// - Widget system for scrollable views and UI components
/// - Remote client for bevy_remote communication
/// - Theme system for consistent styling
///
/// The editor automatically sets up all necessary systems, resources, and UI
/// when added to a Bevy app.
#[derive(Default)]
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app
            // Add sub-plugins for modular functionality  
            .add_plugins((
                EntityListPlugin,
                ComponentInspectorPlugin,
                WidgetsPlugin,
            ))
            // Initialize resources
            .init_resource::<EditorState>()
            .init_resource::<RemoteConnection>()
            .init_resource::<ComponentDisplayState>()
            .add_systems(Startup, setup_editor_ui)
            .add_systems(Update, (
                setup_scroll_content_markers,
                refresh_entity_list,
                handle_entity_selection,
                update_entity_button_colors,
                handle_component_inspection,
                handle_expansion_keyboard,
                update_remote_connection,
                update_status_bar,
            ))
            .add_observer(handle_entities_fetched)
            .add_observer(handle_component_data_fetched);
    }
}

// Import UI marker components from our modular structure
use crate::panels::{
    EntityListItem, ComponentInspector, ComponentInspectorContent,
    EntityTree, EntityListArea
};
use crate::widgets::ExpansionButton;

/// Component for status bar
#[derive(Component)]
pub struct StatusBar;

/// Component for marking clickable expansion buttons embedded in text
#[derive(Component)]
pub struct ExpandableText {
    pub entity_id: u32,
    pub expansion_paths: Vec<String>, // All expansion paths in this text
}

/// Marker component for the expansion buttons container
#[derive(Component)]
pub struct ExpansionButtonsContainer;

/// Setup the main editor UI
fn setup_editor_ui(mut commands: Commands) {
    // Setup camera
    commands.spawn(Camera2d);
    
    // Main container
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|parent| {
            // Top status bar with gradient
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(32.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(8.0)),
                        border: UiRect::bottom(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
                    BorderColor::all(Color::srgb(0.45, 0.45, 0.45)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("[!] Disconnected"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        StatusBar,
                    ));
                });

            // Main content area
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    ..default()
                })
                .with_children(|parent| {
                    // Left panel: Entity hierarchy
                    create_entity_panel(parent);
                    
                    // Right panel: Component inspector
                    create_component_panel(parent);
                });
        });
}

fn create_entity_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Px(380.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::right(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.18, 0.18, 0.18)),
            BorderColor::all(Color::srgb(0.35, 0.35, 0.35)),
            EntityTree,
        ))
        .with_children(|parent| {
            // Header with icon-style design
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(44.0),
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.22, 0.22, 0.22)),
                BorderColor::all(Color::srgb(0.4, 0.4, 0.4)),
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("Entities"),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.95, 0.95)),
                ));
            });

            // Scrollable entity list using the new ScrollView widget
            let _entity_scroll_view = ScrollViewBuilder::new()
                .with_background_color(Color::srgb(0.14, 0.14, 0.14))
                .with_border_color(Color::srgb(0.35, 0.35, 0.35))
                .with_padding(UiRect::all(Val::Px(8.0)))
                .with_scroll_sensitivity(15.0)
                .with_scroll_id(1000) // Unique ID for entity panel
                .spawn(parent);
            
            // The content will be added dynamically to the ScrollContent child
        });
}

fn create_component_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.16, 0.16, 0.16)),
            ComponentInspector,
        ))
        .with_children(|parent| {
            // Header with modern styling
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(44.0),
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.22, 0.22, 0.22)),
                BorderColor::all(Color::srgb(0.4, 0.4, 0.4)),
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("Component Inspector"),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.95, 0.95)),
                ));
            });

            // Component details area using the new ScrollView widget
            let _component_scroll_view = ScrollViewBuilder::new()
                .with_background_color(Color::srgb(0.14, 0.14, 0.14))
                .with_border_color(Color::srgb(0.35, 0.35, 0.35))
                .with_padding(UiRect::all(Val::Px(16.0)))
                .with_scroll_sensitivity(20.0)
                .with_scroll_id(2000) // Unique ID for component panel
                .spawn(parent);
            
            // The component content will be added dynamically to the ScrollContent child
        });
}

/// Handle entity selection in the UI
fn handle_entity_selection(
    mut interaction_query: Query<
        (&Interaction, &EntityListItem, &mut BackgroundColor), 
        (Changed<Interaction>, With<Button>)
    >,
    mut editor_state: ResMut<EditorState>,
) {
    for (interaction, list_item, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                // Store the remote entity ID directly
                editor_state.selected_entity_id = Some(list_item.entity_id);
                editor_state.show_components = true;
                *bg_color = BackgroundColor(Color::srgb(0.3, 0.4, 0.5)); // Selected state
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(Color::srgb(0.25, 0.25, 0.25)); // Hover state
            }
            Interaction::None => {
                // Check if this is the selected entity
                if Some(list_item.entity_id) == editor_state.selected_entity_id {
                    *bg_color = BackgroundColor(Color::srgb(0.3, 0.4, 0.5)); // Keep selected
                } else {
                    *bg_color = BackgroundColor(Color::srgb(0.2, 0.2, 0.2)); // Default state
                }
            }
        }
    }
}

/// Handle component inspection updates
fn handle_component_inspection(
    editor_state: Res<EditorState>,
    remote_conn: Res<RemoteConnection>,
    mut commands: Commands,
) {
    if editor_state.is_changed() && editor_state.show_components {
        if let Some(selected_entity_id) = editor_state.selected_entity_id {
            info!("Fetching component data for entity {}", selected_entity_id);
            // Find the entity and get the full component names
            if let Some(selected_entity) = editor_state.entities.iter().find(|e| e.id == selected_entity_id) {
                // Use the full component names for the API request
                if !selected_entity.full_component_names.is_empty() {
                    info!("Using component names: {:?}", selected_entity.full_component_names);
                    match remote_client::try_fetch_component_data_with_names(
                        &remote_conn.base_url, 
                        selected_entity_id, 
                        selected_entity.full_component_names.clone()
                    ) {
                        Ok(component_data) => {
                            info!("Successfully fetched component data for entity {}", selected_entity_id);
                            commands.trigger(ComponentDataFetched {
                                entity_id: selected_entity_id,
                                component_data,
                            });
                        }
                        Err(err) => {
                            warn!("Failed to fetch component data for entity {}: {}", selected_entity_id, err);
                            // Fall back to showing just the component names
                            let fallback_data = format!(
                                "Component names for Entity {}:\n\n{}",
                                selected_entity_id,
                                selected_entity.components.join("\n")
                            );
                            commands.trigger(ComponentDataFetched {
                                entity_id: selected_entity_id,
                                component_data: fallback_data,
                            });
                        }
                    }
                } else {
                    // No components to fetch
                    commands.trigger(ComponentDataFetched {
                        entity_id: selected_entity_id,
                        component_data: "This entity has no components.".to_string(),
                    });
                }
            }
        }
    }
}



/// Handle entities fetched event
fn handle_entities_fetched(
    trigger: On<EntitiesFetched>,
    mut editor_state: ResMut<EditorState>,
) {
    editor_state.entities = trigger.event().entities.clone();
    editor_state.connection_status = ConnectionStatus::Connected;
}

/// Handle component data fetched event  
fn handle_component_data_fetched(
    trigger: On<ComponentDataFetched>,
    mut commands: Commands,
    component_inspector_query: Query<Entity, (With<ScrollContent>, With<ComponentInspectorContent>)>,
    display_state: Res<ComponentDisplayState>,
) {
    let event = trigger.event();
    
    // Find the component inspector scroll content area specifically
    for content_entity in &component_inspector_query {
        // Clear existing content
        commands.entity(content_entity).despawn_children();
        
        // Build new widget-based content
        commands.entity(content_entity).with_children(|parent| {
            if event.component_data.trim().is_empty() {
                parent.spawn((
                    Text::new(format!("Entity {} - No Component Data\n\nNo component data received from server.", event.entity_id)),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.65, 0.65, 0.65)),
                ));
            } else {
                // Build interactive component widgets
                build_component_widgets(parent, event.entity_id, &event.component_data, &display_state);
            }
        });
    }
}

/// Update remote connection and fetch data
fn update_remote_connection(
    time: Res<Time>,
    mut remote_conn: ResMut<RemoteConnection>,
    mut editor_state: ResMut<EditorState>,
    mut commands: Commands,
) {
    let current_time = time.elapsed_secs_f64();
    
    if current_time - remote_conn.last_fetch >= remote_conn.fetch_interval {
        remote_conn.last_fetch = current_time;
        
        // Update status to show we're attempting to connect
        if editor_state.connection_status == ConnectionStatus::Disconnected {
            editor_state.connection_status = ConnectionStatus::Connecting;
        }
        
        // Try to fetch entities using the remote client framework
        match remote_client::try_fetch_entities(&remote_conn.base_url) {
            Ok(entities) => {
                info!("Successfully fetched {} entities from remote server", entities.len());
                commands.trigger(EntitiesFetched { entities });
                editor_state.connection_status = ConnectionStatus::Connected;
            }
            Err(err) => {
                warn!("Failed to fetch entities: {}", err);
                // Only set error status if we're not already showing disconnected
                if editor_state.connection_status != ConnectionStatus::Disconnected {
                    editor_state.connection_status = ConnectionStatus::Error(err);
                }
                // Clear entities when connection fails
                if !editor_state.entities.is_empty() {
                    editor_state.entities.clear();
                    editor_state.selected_entity_id = None;
                    editor_state.show_components = false;
                }
            }
        }
    }
}

/// TODO: Real bevy_remote integration framework
/// This module will contain the actual HTTP client integration when implemented
mod remote_client {
    use super::*;
    use bevy::remote::{
        builtin_methods::{
            BrpQuery, BrpQueryFilter, BrpQueryParams, ComponentSelector, BRP_QUERY_METHOD,
        },
        BrpRequest,
    };
    
    /// Attempts to connect to a bevy_remote server and fetch entity data
    pub fn try_fetch_entities(base_url: &str) -> Result<Vec<RemoteEntity>, String> {
        // Create a query to get all entities with their components
        let query_request = BrpRequest {
            jsonrpc: "2.0".to_string(),
            method: BRP_QUERY_METHOD.to_string(),
            id: Some(serde_json::to_value(1).map_err(|e| format!("JSON error: {}", e))?),
            params: Some(
                serde_json::to_value(BrpQueryParams {
                    data: BrpQuery {
                        components: Vec::default(), // Get all components
                        option: ComponentSelector::All,
                        has: Vec::default(),
                    },
                    strict: false,
                    filter: BrpQueryFilter::default(),
                })
                .map_err(|e| format!("Failed to serialize query params: {}", e))?,
            ),
        };
        
        // Make the HTTP request
        let response = ureq::post(base_url)
            .timeout(std::time::Duration::from_secs(2))
            .send_json(&query_request)
            .map_err(|e| format!("HTTP request failed: {}", e))?;
        
        // Parse the response as JSON first
        let json_response: serde_json::Value = response
            .into_json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        // Check if we have an error or result
        if let Some(error) = json_response.get("error") {
            return Err(format!("Server error: {}", error));
        }
        
        if let Some(result) = json_response.get("result") {
            parse_brp_entities(result.clone())
        } else {
            Err("No result or error in response".to_string())
        }
    }
    
    /// Fetch component data for a specific entity with explicit component names
    pub fn try_fetch_component_data_with_names(base_url: &str, entity_id: u32, component_names: Vec<String>) -> Result<String, String> {
        // Create a get request for specific entity with component names
        let get_request = BrpRequest {
            jsonrpc: "2.0".to_string(),
            method: "bevy/get".to_string(),
            id: Some(serde_json::to_value(2).map_err(|e| format!("JSON error: {}", e))?),
            params: Some(
                serde_json::json!({
                    "entity": entity_id as u64,
                    "components": component_names
                })
            ),
        };
        
        let response = ureq::post(base_url)
            .timeout(std::time::Duration::from_secs(2))
            .send_json(&get_request)
            .map_err(|e| format!("HTTP request failed: {}", e))?;
        
        let json_response: serde_json::Value = response
            .into_json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        // Check if we have an error or result
        if let Some(error) = json_response.get("error") {
            return Err(format!("Server error: {}", error));
        }
        
        if let Some(result) = json_response.get("result") {
            Ok(serde_json::to_string_pretty(result)
                .unwrap_or_else(|_| "Failed to format component data".to_string()))
        } else {
            Err("No result or error in response".to_string())
        }
    }
    
    /// Parse BRP query response into our RemoteEntity format
    fn parse_brp_entities(result: serde_json::Value) -> Result<Vec<RemoteEntity>, String> {
        let mut entities = Vec::new();
        
        if let Some(entity_array) = result.as_array() {
            for entity_obj in entity_array {
                if let Some(entity_data) = entity_obj.as_object() {
                    // Get entity ID
                    let entity_id = entity_data
                        .get("entity")
                        .and_then(|v| v.as_u64())
                        .ok_or("Missing or invalid entity ID")?;
                    
                    // Extract component names
                    let mut components = Vec::new();
                    let mut full_component_names = Vec::new();
                    if let Some(components_obj) = entity_data.get("components").and_then(|v| v.as_object()) {
                        for component_name in components_obj.keys() {
                            // Store the full name for API calls
                            full_component_names.push(component_name.clone());
                            // Clean up component names (remove module paths for readability)
                            let clean_name = component_name
                                .split("::")
                                .last()
                                .unwrap_or(component_name)
                                .to_string();
                            components.push(clean_name);
                        }
                    }
                    
                    entities.push(RemoteEntity {
                        id: entity_id as u32,
                        components,
                        full_component_names,
                    });
                }
            }
        } else {
            return Err("Expected array of entities in response".to_string());
        }
        
        Ok(entities)
    }
}



/// Refresh the entity list display
fn refresh_entity_list(
    editor_state: Res<EditorState>,
    mut commands: Commands,
    entity_list_area_query: Query<Entity, With<EntityListArea>>,
    list_items_query: Query<Entity, With<EntityListItem>>,
    mut local_entity_count: Local<usize>,
) {
    // Only refresh when the actual entity count changes, not on every state change
    let current_count = editor_state.entities.len();
    if *local_entity_count == current_count {
        return;
    }
    *local_entity_count = current_count;

    // Clear existing list items
    for entity in &list_items_query {
        commands.entity(entity).despawn();
    }

    // Find the entity list area and add new items
    for list_area_entity in entity_list_area_query.iter() {
        // Clear children by despawning them
        commands.entity(list_area_entity).despawn_children();
        
        commands.entity(list_area_entity).with_children(|parent| {
            if editor_state.entities.is_empty() {
                // Show empty state
                parent.spawn((
                    Text::new("No entities connected.\nStart a bevy_remote server to see entities."),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.6, 0.6)),
                    Node {
                        padding: UiRect::all(Val::Px(16.0)),
                        ..default()
                    },
                ));
            } else {
                // Add entity items
                for remote_entity in &editor_state.entities {
                    create_entity_list_item(parent, remote_entity, &editor_state);
                }
            }
        });
    }
}

fn create_entity_list_item(parent: &mut ChildSpawnerCommands, remote_entity: &RemoteEntity, editor_state: &EditorState) {
    // Determine the correct background color based on selection state
    let bg_color = if Some(remote_entity.id) == editor_state.selected_entity_id {
        Color::srgb(0.3, 0.4, 0.5) // Selected state
    } else {
        Color::srgb(0.2, 0.2, 0.2) // Default state
    };

    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(32.0),
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(10.0)),
                margin: UiRect::bottom(Val::Px(2.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(bg_color),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
            EntityListItem { entity_id: remote_entity.id },
        ))
        .with_children(|parent| {
            // Entity icon and name
            parent.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
            )).with_children(|parent| {
                parent.spawn((
                    Text::new(format!("Entity {}", remote_entity.id)),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                ));
            });
        });
}

/// Create status bar with connection info
fn update_status_bar(
    editor_state: Res<EditorState>,
    remote_conn: Res<RemoteConnection>,
    mut status_query: Query<&mut Text, With<StatusBar>>,
) {
    for mut text in &mut status_query {
        let status_text = match &editor_state.connection_status {
            ConnectionStatus::Disconnected => format!("[!] Disconnected from {}", remote_conn.base_url),
            ConnectionStatus::Connecting => format!("[~] Connecting to {}...", remote_conn.base_url),
            ConnectionStatus::Connected => format!("[*] Connected - {} entities", editor_state.entities.len()),
            ConnectionStatus::Error(err) => format!("[!] Error: {}", err),
        };
        text.0 = status_text;
    }
}

/// Update entity button colors based on selection state
fn update_entity_button_colors(
    editor_state: Res<EditorState>,
    mut button_query: Query<(&EntityListItem, &mut BackgroundColor, &Interaction), With<Button>>,
) {
    if !editor_state.is_changed() {
        return;
    }

    for (list_item, mut bg_color, interaction) in &mut button_query {
        // Don't override hover state
        if *interaction == Interaction::Hovered {
            continue;
        }
        
        // Update color based on selection state
        let new_color = if Some(list_item.entity_id) == editor_state.selected_entity_id {
            Color::srgb(0.3, 0.4, 0.5) // Selected state
        } else {
            Color::srgb(0.2, 0.2, 0.2) // Default state
        };
        *bg_color = BackgroundColor(new_color);
    }
}

/// Handle keyboard shortcuts for expansion - improved to work with any component
fn handle_expansion_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut display_state: ResMut<ComponentDisplayState>,
    editor_state: Res<EditorState>,
    remote_conn: Res<RemoteConnection>,
    mut commands: Commands,
) {
    let mut should_refresh = false;
    
    if keyboard.just_pressed(KeyCode::KeyE) {
        // Smart expand: find and expand all top-level expandable fields
        if let Some(selected_entity_id) = editor_state.selected_entity_id {
            if let Some(selected_entity) = editor_state.entities.iter().find(|e| e.id == selected_entity_id) {
                // Generate potential expansion paths for common component fields
                let mut expansion_paths = Vec::new();
                
                for component_name in &selected_entity.components {
                    // Add common expandable field patterns
                    let common_fields = vec![
                        "translation", "rotation", "scale", // Transform
                        "position", "velocity", "acceleration", // Physics
                        "color", "material", "mesh", // Rendering
                        "transform", "global_transform", // Transforms
                        "children", "parent", // Hierarchy
                        "handle", "handles", // Assets
                    ];
                    
                    for field in common_fields {
                        expansion_paths.push(format!("{}.{}", component_name, field));
                    }
                }
                
                // Add paths to expansion state
                for path in expansion_paths {
                    display_state.expanded_paths.insert(path);
                }
                
                should_refresh = true;
                info!("Expanded common component fields for entity {}", selected_entity_id);
            }
        }
    }
    
    if keyboard.just_pressed(KeyCode::KeyC) {
        display_state.expanded_paths.clear();
        should_refresh = true;
        info!("Collapsed all fields");
    }
    
    if keyboard.just_pressed(KeyCode::KeyT) {
        // Toggle specific Transform fields
        if let Some(selected_entity_id) = editor_state.selected_entity_id {
            if let Some(_selected_entity) = editor_state.entities.iter().find(|e| e.id == selected_entity_id) {
                let transform_paths = vec![
                    "Transform.translation".to_string(),
                    "Transform.rotation".to_string(),
                    "Transform.scale".to_string(),
                ];
                
                // Check if any Transform fields are expanded
                let any_transform_expanded = transform_paths.iter()
                    .any(|path| display_state.expanded_paths.contains(path));
                
                if any_transform_expanded {
                    // Collapse Transform fields
                    for path in &transform_paths {
                        display_state.expanded_paths.remove(path);
                    }
                    info!("Collapsed Transform fields");
                } else {
                    // Expand Transform fields
                    for path in transform_paths {
                        display_state.expanded_paths.insert(path);
                    }
                    info!("Expanded Transform fields");
                }
                
                should_refresh = true;
            }
        }
    }
    
    // Refresh component display if expansion state changed
    if should_refresh {
        if let Some(selected_entity_id) = editor_state.selected_entity_id {
            if let Some(selected_entity) = editor_state.entities.iter().find(|e| e.id == selected_entity_id) {
                if !selected_entity.full_component_names.is_empty() {
                    match remote_client::try_fetch_component_data_with_names(
                        &remote_conn.base_url, 
                        selected_entity_id, 
                        selected_entity.full_component_names.clone()
                    ) {
                        Ok(component_data) => {
                            commands.trigger(ComponentDataFetched {
                                entity_id: selected_entity_id,
                                component_data,
                            });
                        }
                        Err(_) => {
                            let fallback_data = format!(
                                "Component names for Entity {}:\n\n{}",
                                selected_entity_id,
                                selected_entity.components.join("\n")
                            );
                            commands.trigger(ComponentDataFetched {
                                entity_id: selected_entity_id,
                                component_data: fallback_data,
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Build component display as interactive widgets instead of just text
fn build_component_widgets(
    parent: &mut ChildSpawnerCommands,
    entity_id: u32,
    components_data: &str,
    display_state: &ComponentDisplayState,
) {
    // Try to parse the JSON response
    if let Ok(json_value) = serde_json::from_str::<Value>(components_data) {
        if let Some(components_obj) = json_value.get("components").and_then(|v| v.as_object()) {
            // Header
            parent.spawn((
                Text::new(format!("Entity {} - Components", entity_id)),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                Node {
                    margin: UiRect::bottom(Val::Px(12.0)),
                    ..default()
                },
            ));
            
            for (component_name, component_data) in components_obj {
                // Clean component name (remove module path)
                let clean_name = component_name.split("::").last().unwrap_or(component_name);
                build_component_widget(parent, clean_name, component_data, component_name, display_state);
            }
            return;
        }
    }
    
    // Fallback to simple text display if parsing fails
    parent.spawn((
        Text::new(format!("Entity {} - Component Data\n\n{}", entity_id, components_data)),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
    ));
}

/// Extract package name from a full component type string
fn extract_package_name(full_component_name: &str) -> String {
    // Handle different patterns:
    // bevy_transform::components::Transform -> bevy_transform
    // bevy_ui::ui_node::Node -> bevy_ui  
    // cube::server::SomeComponent -> cube
    // std::collections::HashMap -> std
    // MyComponent -> MyComponent (no package)
    
    if let Some(first_separator) = full_component_name.find("::") {
        let package_part = &full_component_name[..first_separator];
        
        // Handle cases where the first part might be a crate prefix
        // like "bevy_core" or just "bevy"
        if package_part.contains('_') || package_part.len() <= 12 {
            format!("[{}]", package_part)
        } else {
            // For very long first parts, just take first word
            format!("[{}]", package_part.split('_').next().unwrap_or(package_part))
        }
    } else {
        // No package separator, use the component name itself
        format!("[{}]", full_component_name.split('<').next().unwrap_or(full_component_name))
    }
}

/// Build a single component widget with expansion capabilities
fn build_component_widget(
    parent: &mut ChildSpawnerCommands,
    clean_name: &str,
    component_data: &Value,
    full_component_name: &str,
    display_state: &ComponentDisplayState,
) {
    // Component header container
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            margin: UiRect::bottom(Val::Px(8.0)),
            ..default()
        },
    )).with_children(|parent| {
        // Component title with package name
        let package_name = extract_package_name(full_component_name);
        parent.spawn((
            Text::new(format!("{} {}", package_name, clean_name)),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.95, 0.95)),
            Node {
                margin: UiRect::bottom(Val::Px(4.0)),
                ..default()
            },
        ));
        
        // Build component fields
        let fields = parse_component_fields(full_component_name, component_data);
        for field in fields {
            build_field_widget(parent, &field, 1, &format!("{}.{}", clean_name, field.name), display_state);
        }
    });
}

/// Build a field widget with expansion button if needed
fn build_field_widget(
    parent: &mut ChildSpawnerCommands,
    field: &ComponentField,
    indent_level: usize,
    path: &str,
    display_state: &ComponentDisplayState,
) {
    let indent_px = (indent_level as f32) * 16.0;
    
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            margin: UiRect::left(Val::Px(indent_px)),
            padding: UiRect::all(Val::Px(2.0)),
            ..default()
        },
    )).with_children(|parent| {
        if field.is_expandable {
            let is_expanded = display_state.expanded_paths.contains(path);
            
            // Expansion button
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(18.0),
                    height: Val::Px(18.0),
                    margin: UiRect::right(Val::Px(6.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                BorderColor::all(Color::srgb(0.5, 0.5, 0.5)),
                ExpansionButton {
                    path: path.to_string(),
                    is_expanded,
                },
            )).with_children(|parent| {
                parent.spawn((
                    Text::new(if is_expanded { "-" } else { "+" }),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                ));
            });
            
            // Field name and summary
            let value_summary = format_value_inline(&field.value);
            parent.spawn((
                Text::new(format!("{}: {}", field.name, value_summary)),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        } else {
            // No expansion button for simple values
            parent.spawn((
                Node {
                    width: Val::Px(16.0),
                    height: Val::Px(16.0),
                    margin: UiRect::right(Val::Px(6.0)),
                    ..default()
                },
            ));
            
            // Simple field display
            let formatted_value = format_simple_value(&field.value);
            parent.spawn((
                Text::new(format!("{}: {}", field.name, formatted_value)),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        }
    });
    
    // Show expanded children if the field is expanded
    if field.is_expandable && display_state.expanded_paths.contains(path) {
        if matches!(field.value, Value::Object(_)) {
            build_expanded_object_widgets(parent, &field.value, indent_level + 1, path, display_state);
        } else if matches!(field.value, Value::Array(_)) {
            build_expanded_array_widgets(parent, &field.value, indent_level + 1);
        }
    }
}

/// Build widgets for expanded object fields
fn build_expanded_object_widgets(
    parent: &mut ChildSpawnerCommands,
    value: &Value,
    indent_level: usize,
    path: &str,
    display_state: &ComponentDisplayState,
) {
    let indent_px = (indent_level as f32) * 16.0;
    
    if let Some(obj) = value.as_object() {
        // Check for common Bevy types (Vec3, Vec2, Color, etc.)
        if let (Some(x), Some(y), Some(z)) = (obj.get("x"), obj.get("y"), obj.get("z")) {
            if all_numbers(&[x, y, z]) {
                for (component, val) in [("x", x), ("y", y), ("z", z)] {
                    parent.spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            margin: UiRect::left(Val::Px(indent_px)),
                            padding: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                    )).with_children(|parent| {
                        parent.spawn((
                            Text::new(format!("{}: {:.3}", component, val.as_f64().unwrap_or(0.0))),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.7, 0.9, 0.7)),
                        ));
                    });
                }
                if let Some(w) = obj.get("w") {
                    if w.is_number() {
                        parent.spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                margin: UiRect::left(Val::Px(indent_px)),
                                padding: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                        )).with_children(|parent| {
                            parent.spawn((
                                Text::new(format!("w: {:.3}", w.as_f64().unwrap_or(0.0))),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.7, 0.9, 0.7)),
                            ));
                        });
                    }
                }
                return;
            }
        } else if let (Some(x), Some(y)) = (obj.get("x"), obj.get("y")) {
            if all_numbers(&[x, y]) {
                for (component, val) in [("x", x), ("y", y)] {
                    parent.spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            margin: UiRect::left(Val::Px(indent_px)),
                            padding: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                    )).with_children(|parent| {
                        parent.spawn((
                            Text::new(format!("{}: {:.3}", component, val.as_f64().unwrap_or(0.0))),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.7, 0.9, 0.7)),
                        ));
                    });
                }
                return;
            }
        }
        
        // Generic object handling
        for (key, val) in obj {
            let child_path = format!("{}.{}", path, key);
            let is_simple = is_simple_value(val);
            
            parent.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    margin: UiRect::left(Val::Px(indent_px)),
                    padding: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
            )).with_children(|parent| {
                if is_simple {
                    parent.spawn((
                        Text::new(format!("{}: {}", key, format_simple_value(val))),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.9)),
                    ));
                } else {
                    let is_expanded = display_state.expanded_paths.contains(&child_path);
                    parent.spawn((
                        Text::new(format!("{}{}: {}", 
                            if is_expanded { "[-] " } else { "[+] " },
                            key, 
                            format_value_inline(val)
                        )),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));
                }
            });
        }
    }
}

/// Build widgets for expanded array fields  
fn build_expanded_array_widgets(
    parent: &mut ChildSpawnerCommands,
    value: &Value,
    indent_level: usize,
) {
    let indent_px = (indent_level as f32) * 16.0;
    
    if let Some(arr) = value.as_array() {
        if arr.len() <= 4 && arr.iter().all(|v| v.is_number()) {
            // Small numeric arrays (Vec2, Vec3, Vec4, Quat components)
            for (i, item) in arr.iter().enumerate() {
                let comp_name = match i {
                    0 => "x", 1 => "y", 2 => "z", 3 => "w",
                    _ => &format!("[{}]", i),
                };
                parent.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        margin: UiRect::left(Val::Px(indent_px)),
                        padding: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                )).with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("{}: {:.3}", comp_name, item.as_f64().unwrap_or(0.0))),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.9, 0.7)),
                    ));
                });
            }
        } else if arr.len() <= 10 {
            // Small arrays - show all items
            for (i, item) in arr.iter().enumerate() {
                parent.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        margin: UiRect::left(Val::Px(indent_px)),
                        padding: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                )).with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("[{}]: {}", i, format_simple_value(item))),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.7, 0.7)),
                    ));
                });
            }
        } else {
            // Large arrays - show first few items
            for (i, item) in arr.iter().take(3).enumerate() {
                parent.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        margin: UiRect::left(Val::Px(indent_px)),
                        padding: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                )).with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("[{}]: {}", i, format_simple_value(item))),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.7, 0.7)),
                    ));
                });
            }
            parent.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    margin: UiRect::left(Val::Px(indent_px)),
                    padding: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
            )).with_children(|parent| {
                parent.spawn((
                    Text::new(format!("... ({} more items)", arr.len() - 3)),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.6, 0.6)),
                ));
            });
        }
    }
}

/// System to setup marker components on ScrollContent areas
/// This ensures the existing entity list and component inspector systems can find their target areas
fn setup_scroll_content_markers(
    mut commands: Commands,
    scroll_content_query: Query<Entity, (With<ScrollContent>, Without<EntityListArea>, Without<ComponentInspectorContent>)>,
    mut has_run: Local<bool>,
) {
    // Only run once to avoid repeatedly adding components
    if *has_run {
        return;
    }
    
    let scroll_content_entities: Vec<Entity> = scroll_content_query.iter().collect();
    
    if scroll_content_entities.len() >= 2 {
        // Mark the first ScrollContent as EntityListArea (entity panel is created first)
        commands.entity(scroll_content_entities[0]).insert(EntityListArea);
        
        // Mark the second ScrollContent as ComponentInspectorContent (component panel is created second)
        commands.entity(scroll_content_entities[1]).insert(ComponentInspectorContent);
        
        *has_run = true;
        info!("Added marker components to ScrollContent areas");
    }
}
