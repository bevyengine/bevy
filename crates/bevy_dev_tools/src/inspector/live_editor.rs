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
use bevy_core::Name;
use bevy_core_pipeline::core_2d::Camera2d;
use bevy_ecs::prelude::*;
use bevy_input::{keyboard::KeyCode, ButtonInput};
use bevy_render::camera::{Camera, RenderTarget};
use bevy_text::{Text, TextColor, TextFont};
use bevy_transform::prelude::Transform;
use bevy_ui::prelude::*;
use bevy_window::{Window, WindowRef};
use bevy_color::Color;
use tracing::info;

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
                provide_local_world_data,
                setup_editor_ui_in_window,
            ));
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
    editor_state: Res<LiveEditorState>,
    window_query: Query<Entity, (With<LiveEditorWindow>, Added<Window>)>,
) {
    // Only setup UI when the window is first created
    for window_entity in window_query.iter() {
        // Create camera for the editor window
        let camera_entity = commands.spawn((
            Camera2dBundle {
                camera: Camera {
                    target: RenderTarget::Window(WindowRef::Entity(window_entity)),
                    ..Default::default()
                },
            },
        )).id();
        
        // Create the root container and setup the editor UI structure
        // This mirrors the setup in EditorPlugin but targets our camera
        commands
            .spawn((
                bevy_ui::NodeBundle {
                    style: bevy_ui::Style {
                        width: bevy_ui::Val::Percent(100.0),
                        height: bevy_ui::Val::Percent(100.0),
                        flex_direction: bevy_ui::FlexDirection::Column,
                        ..Default::default()
                    },
                    background_color: bevy_color::Color::srgb(0.1, 0.1, 0.1).into(),
                    ..Default::default()
                },
                bevy_ui::UiTargetCamera(camera_entity),
            ))
            .with_children(|parent| {
                // Create status bar
                parent.spawn((
                    bevy_ui::NodeBundle {
                        style: bevy_ui::Style {
                            width: bevy_ui::Val::Percent(100.0),
                            height: bevy_ui::Val::Px(32.0),
                            align_items: bevy_ui::AlignItems::Center,
                            padding: bevy_ui::UiRect::horizontal(bevy_ui::Val::Px(12.0)),
                            border: bevy_ui::UiRect::bottom(bevy_ui::Val::Px(1.0)),
                            ..Default::default()
                        },
                        background_color: bevy_color::Color::srgb(0.15, 0.15, 0.15).into(),
                        border_color: bevy_color::Color::srgb(0.3, 0.3, 0.3).into(),
                        ..Default::default()
                    },
                    super::editor::StatusBar,
                )).with_children(|parent| {
                    parent.spawn((
                        bevy_text::TextBundle::from_section(
                            "Bevy Live Inspector - Direct World Access",
                            bevy_text::TextStyle {
                                font_size: 14.0,
                                color: bevy_color::Color::srgb(0.8, 0.8, 0.8),
                                ..Default::default()
                            }
                        ),
                    ));
                });

                // Main content area with two panels
                parent.spawn((
                    bevy_ui::NodeBundle {
                        style: bevy_ui::Style {
                            width: bevy_ui::Val::Percent(100.0),
                            height: bevy_ui::Val::Percent(100.0),
                            flex_direction: bevy_ui::FlexDirection::Row,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )).with_children(|parent| {
                    // Create entity panel using the existing function
                    super::editor::create_entity_panel(parent);
                    
                    // Create component panel using the existing function  
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
    mut editor_state: ResMut<EditorState>,
    // Query all entities from the world
    all_entities: Query<Entity>,
    name_query: Query<&Name>,
    transform_query: Query<&Transform>,
    mut events: EventWriter<EntitiesFetched>,
    mut component_events: EventWriter<ComponentDataFetched>,
) {
    let current_time = time.elapsed_seconds_f64();
    
    // Throttle updates to avoid overwhelming the UI
    if current_time - data_provider.last_update < data_provider.update_frequency {
        return;
    }
    data_provider.last_update = current_time;
    
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
    
    // Send entities to the EditorPlugin
    events.send(EntitiesFetched {
        entities: remote_entities,
    });
    
    // If there's a selected entity, provide its component data
    if let Some(selected_id) = editor_state.selected_entity_id {
        if let Some(entity) = all_entities.iter().find(|e| e.index() == selected_id) {
            let mut component_data = Vec::new();
            
            // Add Name component data
            if let Ok(name) = name_query.get(entity) {
                component_data.push(format!("Name: {}", name.as_str()));
            }
            
            // Add Transform component data
            if let Ok(transform) = transform_query.get(entity) {
                component_data.push(format!(
                    "Transform:\n  Translation: {:.2}, {:.2}, {:.2}\n  Rotation: {:.2}, {:.2}, {:.2}, {:.2}\n  Scale: {:.2}, {:.2}, {:.2}",
                    transform.translation.x, transform.translation.y, transform.translation.z,
                    transform.rotation.x, transform.rotation.y, transform.rotation.z, transform.rotation.w,
                    transform.scale.x, transform.scale.y, transform.scale.z
                ));
            }
            
            component_events.send(ComponentDataFetched {
                entity_id: selected_id,
                component_data: component_data.join("\n\n"),
            });
        }
    }
}
