//! # Live Editor Plugin
//!
//! This module provides a live editor that opens in a new window for real-time inspection
//! of the running Bevy world. Rather than recreating the UI from scratch, it demonstrates
//! how the existing EditorPlugin could be adapted to work with local world data.
//!
//! ## Features
//!
//! - **Separate Window**: Opens in its own window like an external editor
//! - **Live Inspection**: Direct access to the running world without remote connections
//! - **F3 Toggle**: Simple toggle like FPS overlay
//! - **EditorPlugin-Style UI**: Uses the same patterns and styling as the full EditorPlugin
//!
//! ## Usage
//!
//! Add the plugin to your app:
//! ```rust,no_run
//! use bevy::prelude::*;
//! use bevy_dev_tools::inspector::LiveEditorPlugin;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(LiveEditorPlugin::default())
//!     .run();
//! ```

use bevy_app::{App, Plugin, Startup, Update};
use bevy_ecs::{prelude::*, name::Name};
use bevy_input::{keyboard::KeyCode, ButtonInput};
use bevy_render::camera::{Camera, RenderTarget};
use bevy_text::{TextColor, TextFont};
use bevy_transform::prelude::Transform;
use bevy_ui::prelude::*;
use bevy_window::{Window, WindowRef};
use bevy_color::Color;
use bevy_time::Time;
use core::default::Default;
use tracing::info;

use crate::inspector::{
    panels::{ComponentDisplayState, ComponentInspectorPlugin, EditorState, EntityListPlugin, EntityListArea, ComponentInspectorContent, EntityListViewMode}, 
    widgets::{WidgetsPlugin, ScrollContent},
    remote::types::{RemoteEntity, ComponentDataFetched, EntitiesFetched},
    editor::{setup_scroll_content_markers, handle_entities_fetched}
};

/// Configuration for the Live Editor Plugin
#[derive(Resource, Clone)]
pub struct LiveEditorConfig {
    /// Whether the editor window starts open
    pub enabled: bool,
    /// Key to toggle the editor window
    pub toggle_key: Option<KeyCode>,
    /// Window title for the editor
    pub window_title: String,
    /// Initial window size
    pub window_size: (f32, f32),
}

impl Default for LiveEditorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            toggle_key: Some(KeyCode::F3),
            window_title: "Bevy Live Inspector".to_string(),
            window_size: (1200.0, 800.0),
        }
    }
}

/// A plugin that adds a live editor window for real-time world inspection.
/// 
/// This plugin creates a separate window containing the existing EditorPlugin
/// interface but adapted for direct world access without remote connections.
/// It provides the same sophisticated UI and functionality as EditorPlugin
/// but opens in a new window and works with local world data.
#[derive(Default)]
pub struct LiveEditorPlugin {
    /// Configuration for the editor
    pub config: LiveEditorConfig,
}

impl LiveEditorPlugin {
    /// Create a new LiveEditorPlugin with default configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set whether the editor starts enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }
    
    /// Set the toggle key for the editor
    pub fn with_toggle_key(mut self, key: KeyCode) -> Self {
        self.config.toggle_key = Some(key);
        self
    }
    
    /// Set the window title
    pub fn with_window_title(mut self, title: String) -> Self {
        self.config.window_title = title;
        self
    }
}

impl Plugin for LiveEditorPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(self.config.clone())
            .init_resource::<LiveEditorState>()
            // Add the required events
            .add_event::<EntitiesFetched>()
            .add_event::<ComponentDataFetched>()
            // Add the editor panels and widgets but without the remote client
            .add_plugins((
                EntityListPlugin,
                ComponentInspectorPlugin, 
                WidgetsPlugin,
            ))
            // Initialize editor state for the panels to use
            .init_resource::<EditorState>()
            .init_resource::<ComponentDisplayState>()
            // Create a local data provider
            .init_resource::<LocalWorldDataProvider>()
            .add_systems(Startup, setup_live_editor_window)
            .add_systems(Update, (
                handle_toggle_input,
                update_editor_visibility,
                mark_inspector_ui_entities,
                mark_new_ui_entities_when_live_editor_active,
                provide_local_world_data.after(mark_inspector_ui_entities).after(mark_new_ui_entities_when_live_editor_active),
                setup_editor_ui_in_window,
                setup_scroll_content_markers,
            ))
            .add_observer(handle_entities_fetched);
    }
}

/// Resource to track live editor state
#[derive(Resource)]
pub struct LiveEditorState {
    /// Whether the editor window is currently visible
    pub window_open: bool,
    /// The entity representing the editor window
    pub editor_window: Option<Entity>,
}

impl Default for LiveEditorState {
    fn default() -> Self {
        Self {
            window_open: false,
            editor_window: None,
        }
    }
}

/// Marker component for the live editor window
#[derive(Component)]
pub struct LiveEditorWindow;

/// Marker component for inspector UI entities that should be excluded from inspection
#[derive(Component)]
pub struct InspectorUIEntity;

/// Marker component for the local data provider
#[derive(Component)]
pub struct LocalDataProvider;

/// Resource that provides local world data to the EditorPlugin
/// This replaces the remote connection functionality with direct world access
#[derive(Resource)]
pub struct LocalWorldDataProvider {
    /// Last update time to throttle data updates
    pub last_update: f64,
    /// Update frequency in seconds
    pub update_frequency: f64,
}

impl Default for LocalWorldDataProvider {
    fn default() -> Self {
        Self {
            last_update: 0.0,
            update_frequency: 0.1, // Update 10 times per second
        }
    }
}

/// Setup the live editor window for the editor UI
fn setup_live_editor_window(
    mut commands: Commands,
    config: Res<LiveEditorConfig>,
    mut editor_state: ResMut<LiveEditorState>,
) {
    // Create the editor window that will contain the editor UI
    let window_entity = commands.spawn((
        Window {
            title: config.window_title.clone(),
            resolution: config.window_size.into(),
            visible: config.enabled,
            ..Default::default()
        },
        LiveEditorWindow,
    )).id();
    
    editor_state.editor_window = Some(window_entity);
    editor_state.window_open = config.enabled;
    
    info!("Live Editor window created");
}

/// Setup the editor UI components in the live editor window
/// This reuses the setup logic from EditorPlugin but targets our window
fn setup_editor_ui_in_window(
    mut commands: Commands,
    window_query: Query<Entity, (With<LiveEditorWindow>, Added<Window>)>,
) {
    // Only setup UI when the window is first created
    for window_entity in window_query.iter() {
        // Create camera for the editor window
        let camera_entity = commands.spawn((
            Camera {
                target: RenderTarget::Window(WindowRef::Entity(window_entity)),
                ..Default::default()
            },
            InspectorUIEntity,
        )).id();
        
        // Create the root container and setup the editor UI structure
        // This mirrors the setup in EditorPlugin but targets our camera
        commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                UiTargetCamera(camera_entity),
                InspectorUIEntity,
            ))
            .with_children(|parent| {
                // Create status bar
                parent.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(32.0),
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(Val::Px(12.0)),
                        border: UiRect::bottom(Val::Px(1.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                    super::editor::StatusBar,
                    InspectorUIEntity,
                )).with_children(|parent| {
                    parent.spawn((
                        Text::new("Bevy Live Inspector - Direct World Access"),
                        TextFont { font_size: 14.0, ..Default::default() },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        InspectorUIEntity,
                    ));
                });

                // Main content area with two panels
                parent.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        ..Default::default()
                    },
                    InspectorUIEntity,
                )).with_children(|parent| {
                    // Create entity panel and mark it as inspector UI
                    let entity_panel_id = parent.target_entity();
                    super::editor::create_entity_panel(parent);
                    
                    // Create component panel and mark it as inspector UI  
                    super::editor::create_component_panel(parent);
                });
            });
        
        info!("Live Editor UI setup complete for window {:?}", window_entity);
        break; // Only setup once
    }
}

/// Handle toggle input for the editor window
fn handle_toggle_input(
    mut editor_state: ResMut<LiveEditorState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    config: Res<LiveEditorConfig>,
) {
    if let Some(toggle_key) = config.toggle_key {
        if keyboard.just_pressed(toggle_key) {
            editor_state.window_open = !editor_state.window_open;
            info!("Live Editor toggled: {}", if editor_state.window_open { "ON" } else { "OFF" });
        }
    }
}

/// Update editor window visibility
fn update_editor_visibility(
    editor_state: Res<LiveEditorState>,
    mut window_query: Query<&mut Window, With<LiveEditorWindow>>,
) {
    if editor_state.is_changed() {
        for mut window in window_query.iter_mut() {
            window.visible = editor_state.window_open;
        }
    }
}

/// Provide local world data to the EditorPlugin instead of remote data
/// This system converts local world state into the format expected by EditorPlugin
fn provide_local_world_data(
    mut data_provider: ResMut<LocalWorldDataProvider>,
    time: Res<Time>,
    // Query all entities EXCEPT inspector UI entities, cameras, and live editor windows
    all_entities: Query<Entity, (Without<LiveEditorWindow>, Without<Camera>, Without<InspectorUIEntity>)>,
    name_query: Query<&Name>,
    transform_query: Query<&Transform>,
    mut commands: Commands,
) {
    let current_time = time.elapsed_secs_f64();
    
    // Throttle updates to avoid overwhelming the UI
    if current_time - data_provider.last_update < data_provider.update_frequency {
        return;
    }
    data_provider.last_update = current_time;
    
    // Safety check: limit the number of entities to prevent runaway growth
    let entity_count = all_entities.iter().count();
    if entity_count > 5000 {
        info!("Entity count ({}) exceeds safety limit (5000), skipping update", entity_count);
        return;
    }
    
    info!("Entity filtering: {} entities passed filter (excluded LiveEditorWindow, Camera, InspectorUIEntity)", entity_count);
    
    // Convert local entities to the format expected by EditorPlugin
    let remote_entities: Vec<RemoteEntity> = all_entities
        .iter()
        .map(|entity| {
            let mut components = Vec::new();
            let mut full_component_names = Vec::new();
            
            // Check for Name component
            if name_query.contains(entity) {
                components.push("Name".to_string());
                full_component_names.push("bevy_core::name::Name".to_string());
            }
            
            // Check for Transform component
            if transform_query.contains(entity) {
                components.push("Transform".to_string());
                full_component_names.push("bevy_transform::components::transform::Transform".to_string());
            }
            
            RemoteEntity {
                id: entity.index(),
                components,
                full_component_names,
            }
        })
        .collect();
    
    // Send entities using triggers instead of events
    info!("Triggering EntitiesFetched with {} entities", remote_entities.len());
    commands.trigger(EntitiesFetched {
        entities: remote_entities,
    });
}

/// Mark all UI entities in the live editor window as inspector UI entities
/// This prevents them from being included in the entity inspection
fn mark_inspector_ui_entities(
    mut commands: Commands,
    live_editor_windows: Query<Entity, With<LiveEditorWindow>>,
    ui_target_cameras: Query<&UiTargetCamera>,
    all_ui_entities: Query<Entity, (With<bevy_ui::Node>, Without<InspectorUIEntity>)>,
    camera_entities: Query<(Entity, &Camera)>,
    // Also check for entities that might be indirectly related to the live editor
    all_components: Query<Entity, (Or<(With<crate::inspector::panels::EntityTree>, With<crate::inspector::panels::ComponentInspector>, With<super::editor::StatusBar>)>, Without<InspectorUIEntity>)>,
) {
    if live_editor_windows.is_empty() {
        return; // No live editor windows, nothing to mark
    }
    
    // Find cameras targeting live editor windows
    let mut live_editor_cameras = Vec::new();
    for (camera_entity, camera) in camera_entities.iter() {
        if let RenderTarget::Window(WindowRef::Entity(window_entity)) = &camera.target {
            if live_editor_windows.contains(*window_entity) {
                live_editor_cameras.push(camera_entity);
            }
        }
    }
    
    let mut marked_count = 0;
    
    // Mark all UI entities that target live editor cameras
    for ui_entity in all_ui_entities.iter() {
        if let Ok(ui_target_camera) = ui_target_cameras.get(ui_entity) {
            if live_editor_cameras.contains(&ui_target_camera.0) {
                commands.entity(ui_entity).insert(InspectorUIEntity);
                marked_count += 1;
            }
        }
    }
    
    // Also mark entities with specific inspector components
    for entity in all_components.iter() {
        commands.entity(entity).insert(InspectorUIEntity);
        marked_count += 1;
    }
    
    if marked_count > 0 {
        info!("Marked {} UI entities as InspectorUIEntity", marked_count);
    }
}

/// Aggressively mark any new UI entities when live editor is active 
/// This prevents runaway entity growth from UI generation
fn mark_new_ui_entities_when_live_editor_active(
    mut commands: Commands,
    live_editor_state: Res<LiveEditorState>,
    new_ui_entities: Query<Entity, (With<bevy_ui::Node>, Without<InspectorUIEntity>, Added<bevy_ui::Node>)>,
) {
    // Only run if live editor window is open
    if !live_editor_state.window_open {
        return;
    }
    
    // Mark any newly created UI entities as inspector UI to prevent them from being inspected
    let mut marked_count = 0;
    for entity in new_ui_entities.iter() {
        commands.entity(entity).insert(InspectorUIEntity);
        marked_count += 1;
    }
    
    if marked_count > 0 {
        info!("Aggressively marked {} new UI entities as InspectorUIEntity", marked_count);
    }
}
