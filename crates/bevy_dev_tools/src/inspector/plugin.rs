//! Inspector Plugin

use bevy_app::{App, Plugin, Update};

use super::components::*;
use super::config::*;
use super::data_sources::*;
use super::systems::{
    handle_inspector_toggle, spawn_inspector_window, update_inspector_content,
    handle_disclosure_interactions, handle_entity_selection, handle_detailed_view,
    handle_panel_resize,
};

/// Main inspector plugin that provides entity and component inspection capabilities
/// 
/// This plugin provides:
/// - Reflection-based component inspection
/// - Smart entity grouping and naming
/// - Collapsible UI panels with disclosure triangles
/// - Multiple data source support (local, remote, asset files)
/// - Configurable keyboard shortcuts and UI styling
/// - Self-filtering to avoid infinite inspection loops
///
/// # Usage
/// 
/// ## Basic Setup
/// ```rust
/// use bevy::prelude::*;
/// use bevy_dev_tools::inspector::InspectorPlugin;
/// 
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(InspectorPlugin::default())
///         .run();
/// }
/// ```
/// 
/// ## Custom Configuration
/// ```rust
/// use bevy::prelude::*;
/// use bevy_dev_tools::inspector::{InspectorPlugin, InspectorConfig};
/// use bevy_input::keyboard::KeyCode;
/// 
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(InspectorPlugin::new()
///             .with_toggle_key(KeyCode::F11)
///             .with_window_size(900.0, 1000.0)
///             .with_auto_refresh(1.0))
///         .run();
/// }
/// ```
/// 
/// ## Custom Data Source
/// ```rust
/// use bevy::prelude::*;
/// use bevy_dev_tools::inspector::{InspectorPlugin, RemoteDataSource};
/// 
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(InspectorPlugin::new()
///             .with_data_source(RemoteDataSource::new("http://localhost:8080")))
///         .run();
/// }
/// ```
pub struct InspectorPlugin {
    config: InspectorConfig,
    data_source: Option<Box<dyn InspectorDataSource>>,
}

impl Default for InspectorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl InspectorPlugin {
    /// Create a new inspector plugin with default configuration
    pub fn new() -> Self {
        Self {
            config: InspectorConfig::default(),
            data_source: None,
        }
    }

    /// Quick setup for local world inspection with F12 toggle
    pub fn quick() -> Self {
        Self::default()
    }
    
    /// Quick setup for development with common settings
    pub fn dev() -> Self {
        Self::new()
            .with_toggle_key(bevy_input::keyboard::KeyCode::F12)
            .with_auto_refresh(0.5)
            .with_window_size(900.0, 1000.0)
            .with_title("Bevy Dev Inspector")
    }
    
    /// Quick setup for debugging with more detailed options
    pub fn debug() -> Self {
        Self::new()
            .with_toggle_key(bevy_input::keyboard::KeyCode::F12)
            .with_auto_refresh(0.1) // Fast refresh for debugging
            .with_window_size(1000.0, 1200.0)
            .with_title("Bevy Debug Inspector")
            .with_component_counts(true)
            .with_entity_ids(true)
            .with_max_entities_per_group(100)
    }
    
    /// Set a custom toggle key
    pub fn with_toggle_key(mut self, key: bevy_input::keyboard::KeyCode) -> Self {
        self.config.toggle_key = key;
        self
    }
    
    /// Set custom window dimensions
    pub fn with_window_size(mut self, width: f32, height: f32) -> Self {
        self.config.window_width = width;
        self.config.window_height = height;
        self
    }
    
    /// Set auto-refresh interval in seconds (0.0 to disable)
    pub fn with_auto_refresh(mut self, interval: f32) -> Self {
        self.config.auto_refresh_interval = interval;
        self
    }
    
    /// Set custom window title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.window_title = title.into();
        self
    }
    
    /// Set maximum entities per group before pagination
    pub fn with_max_entities_per_group(mut self, max: usize) -> Self {
        self.config.max_entities_per_group = max;
        self
    }
    
    /// Configure whether to show component count badges
    pub fn with_component_counts(mut self, show: bool) -> Self {
        self.config.show_component_counts = show;
        self
    }
    
    /// Configure whether to show entity IDs
    pub fn with_entity_ids(mut self, show: bool) -> Self {
        self.config.show_entity_ids = show;
        self
    }
    
    /// Set a custom data source
    pub fn with_data_source(mut self, source: impl InspectorDataSource) -> Self {
        self.data_source = Some(Box::new(source));
        self
    }
    
    /// Set custom entity grouping rules
    pub fn with_grouping_rules(mut self, rules: EntityGroupingRules) -> Self {
        self.config.grouping_rules = rules;
        self
    }
    
    /// Set custom UI styling
    pub fn with_styling(mut self, styling: InspectorStyling) -> Self {
        self.config.styling = styling;
        self
    }
    
    /// Create an inspector plugin for remote inspection
    pub fn remote(url: impl Into<String>) -> Self {
        Self::new().with_data_source(RemoteDataSource::new(url))
    }
    
    /// Create an inspector plugin for asset file inspection
    pub fn asset_file(path: impl Into<std::path::PathBuf>) -> Self {
        Self::new().with_data_source(AssetFileDataSource::new(path))
    }
}

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        // Insert resources
        app.insert_resource(self.config.clone())
            .insert_resource(InspectorState::new());
        
        // Insert data source
        let data_source = if let Some(source) = &self.data_source {
            // Clone the concrete type based on its actual type
            if let Some(local_source) = source.as_any().downcast_ref::<LocalWorldDataSource>() {
                InspectorDataSourceResource {
                    source: Box::new(local_source.clone()),
                }
            } else if let Some(remote_source) = source.as_any().downcast_ref::<RemoteDataSource>() {
                InspectorDataSourceResource {
                    source: Box::new(remote_source.clone()),
                }
            } else if let Some(asset_source) = source.as_any().downcast_ref::<AssetFileDataSource>() {
                InspectorDataSourceResource {
                    source: Box::new(asset_source.clone()),
                }
            } else {
                // Fallback to default if we can't downcast
                InspectorDataSourceResource {
                    source: Box::new(LocalWorldDataSource),
                }
            }
        } else {
            InspectorDataSourceResource {
                source: Box::new(LocalWorldDataSource),
            }
        };
        app.insert_resource(data_source);

        // Add systems - split into multiple calls to avoid tuple limit
        app.add_systems(
            Update,
            (
                handle_inspector_toggle,
                spawn_inspector_window,
            ),
        );
        
        app.add_systems(
            Update,
            update_inspector_content,
        );
        
        app.add_systems(
            Update,
            (
                handle_disclosure_interactions,
                handle_entity_selection,
                handle_detailed_view,
                handle_panel_resize,
            ),
        );
    }
}

// Implement Clone for our data sources
impl Clone for LocalWorldDataSource {
    fn clone(&self) -> Self {
        Self
    }
}

impl Clone for RemoteDataSource {
    fn clone(&self) -> Self {
        Self {
            connection_url: self.connection_url.clone(),
            cached_entities: self.cached_entities.clone(),
            last_update: self.last_update,
        }
    }
}

impl Clone for AssetFileDataSource {
    fn clone(&self) -> Self {
        Self {
            file_path: self.file_path.clone(),
            cached_entities: self.cached_entities.clone(),
        }
    }
}

