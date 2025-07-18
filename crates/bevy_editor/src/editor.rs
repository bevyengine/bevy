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

use crate::remote::{client, RemoteClientPlugin, types::{EditorState, ComponentDisplayState, ComponentDataFetched, RemoteConnection, EntitiesFetched, ConnectionStatus, RemoteEntity}};
use crate::panels::{EntityListPlugin, ComponentInspectorPlugin};
use crate::widgets::{WidgetsPlugin, ScrollViewBuilder, ScrollContent};

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
                RemoteClientPlugin,
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
    ComponentInspector, ComponentInspectorContent,
    EntityTree, EntityListArea
};
use crate::widgets::EntityListItem;

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
                    match client::try_fetch_component_data_with_names(
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
                // Use the modular component inspector implementation
                crate::panels::component_inspector::build_component_widgets(parent, event.entity_id, &event.component_data, &display_state);
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
        match client::try_fetch_entities(&remote_conn.base_url) {
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
            EntityListItem::from_remote_entity(&remote_entity),
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
                    match client::try_fetch_component_data_with_names(
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
