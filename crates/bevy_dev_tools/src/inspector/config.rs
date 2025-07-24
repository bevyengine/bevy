//! Inspector Configuration and Settings

use bevy_ecs::resource::Resource;
use bevy_input::keyboard::KeyCode;
use std::collections::HashMap;

/// Main inspector configuration resource
#[derive(Clone, Resource, Debug)]
pub struct InspectorConfig {
    /// Keyboard shortcut to toggle inspector window
    pub toggle_key: KeyCode,
    /// Inspector window dimensions
    pub window_width: f32,
    pub window_height: f32,
    /// Window title
    pub window_title: String,
    /// Auto-refresh interval in seconds (0.0 = disabled)
    pub auto_refresh_interval: f32,
    /// Maximum entities to show per group before pagination
    pub max_entities_per_group: usize,
    /// Whether to show component count badges
    pub show_component_counts: bool,
    /// Whether to show entity IDs in the display
    pub show_entity_ids: bool,
    /// Custom entity grouping rules
    pub grouping_rules: EntityGroupingRules,
    /// UI styling preferences
    pub styling: InspectorStyling,
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            toggle_key: KeyCode::F12,
            window_width: 800.0,
            window_height: 900.0,
            window_title: "Bevy Entity Inspector".to_string(),
            auto_refresh_interval: 0.5,
            max_entities_per_group: 50,
            show_component_counts: true,
            show_entity_ids: true,
            grouping_rules: EntityGroupingRules::default(),
            styling: InspectorStyling::default(),
        }
    }
}

/// Entity grouping configuration
#[derive(Clone, Debug)]
pub struct EntityGroupingRules {
    /// Priority order for component-based grouping
    pub component_priority: Vec<String>,
    /// Custom group names for specific component combinations
    pub custom_group_names: HashMap<Vec<String>, String>,
    /// Components to ignore when creating groups
    pub ignored_components: Vec<String>,
}

impl Default for EntityGroupingRules {
    fn default() -> Self {
        Self {
            component_priority: vec![
                "Camera".to_string(),
                "Mesh3d".to_string(),
                "DirectionalLight".to_string(),
                "PointLight".to_string(),
                "SpotLight".to_string(),
                "Transform".to_string(),
                "GlobalTransform".to_string(),
                "Name".to_string(),
            ],
            custom_group_names: {
                let mut map = HashMap::new();
                map.insert(
                    vec!["Camera".to_string(), "Transform".to_string()],
                    "Cameras".to_string(),
                );
                map.insert(
                    vec!["Mesh3d".to_string(), "MeshMaterial3d<StandardMaterial>".to_string()],
                    "3D Objects".to_string(),
                );
                map.insert(
                    vec!["DirectionalLight".to_string()],
                    "Directional Lights".to_string(),
                );
                map.insert(
                    vec!["PointLight".to_string()],
                    "Point Lights".to_string(),
                );
                map
            },
            ignored_components: vec![
                "InspectorMarker".to_string(),
                "ComputedVisibility".to_string(),
                "GlobalTransform".to_string(), // Often shown alongside Transform
            ],
        }
    }
}

/// UI styling configuration
#[derive(Clone, Debug)]
pub struct InspectorStyling {
    pub background_color: (f32, f32, f32, f32),
    pub header_color: (f32, f32, f32, f32),
    pub text_color: (f32, f32, f32, f32),
    pub highlight_color: (f32, f32, f32, f32),
    pub font_size_header: f32,
    pub font_size_normal: f32,
    pub font_size_small: f32,
    pub padding: f32,
    pub margin: f32,
}

impl Default for InspectorStyling {
    fn default() -> Self {
        Self {
            background_color: (0.15, 0.15, 0.15, 0.95),
            header_color: (0.25, 0.25, 0.25, 1.0),
            text_color: (0.9, 0.9, 0.9, 1.0),
            highlight_color: (0.3, 0.7, 1.0, 1.0),
            font_size_header: 18.0,
            font_size_normal: 14.0,
            font_size_small: 12.0,
            padding: 8.0,
            margin: 4.0,
        }
    }
}
