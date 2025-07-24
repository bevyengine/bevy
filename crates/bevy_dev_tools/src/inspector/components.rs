//! Inspector Component Markers and State

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use std::collections::{HashMap, HashSet};
use bevy_reflect::Reflect;

/// Marker component to exclude inspector's own entities from being inspected
#[derive(Component, Reflect)]
pub struct InspectorMarker;

/// Component for the main inspector window root
#[derive(Component)]
pub struct InspectorWindowRoot {
    pub window_entity: Entity,
}

/// Component for the inspector tree/list container
#[derive(Component)]
pub struct InspectorTreeRoot;

/// Component for collapsible group headers with disclosure triangles
#[derive(Component, Clone)]
pub struct DisclosureTriangle {
    pub group_id: String,
    pub is_expanded: bool,
    pub entity_count: usize,
}

/// Component for individual entity rows in the inspector
#[derive(Component, Clone)]
pub struct EntityRow {
    pub entity_id: Entity,
    pub display_name: String,
    pub component_names: Vec<String>,
    pub is_selected: bool,
}

/// Component for component detail panels
#[derive(Component)]
pub struct ComponentDetailPanel {
    pub entity_id: Entity,
    pub component_type_name: String,
}

/// Component for scrollable containers
#[derive(Component)]
pub struct ScrollableContainer {
    pub scroll_position: f32,
    pub content_height: f32,
}

/// Resource tracking inspector UI state
#[derive(bevy_ecs::resource::Resource)]
pub struct InspectorState {
    /// Currently expanded groups
    pub expanded_groups: HashSet<String>,
    /// Currently selected entity (for detailed view)
    pub selected_entity: Option<Entity>,
    /// Search filter text
    pub search_filter: String,
    /// Last refresh timestamp
    pub last_refresh: std::time::Instant,
    /// Window visibility state
    pub window_visible: bool,
    /// Current view mode (tree, list, detailed)
    pub view_mode: ViewMode,
    /// Cached entity groupings
    pub entity_groups: HashMap<String, Vec<Entity>>,
    /// Inspector window entity (for separate window)
    pub inspector_window_entity: Option<Entity>,
}

impl InspectorState {
    pub fn new() -> Self {
        Self {
            expanded_groups: HashSet::new(),
            selected_entity: None,
            search_filter: String::new(),
            last_refresh: std::time::Instant::now(),
            window_visible: false,
            view_mode: ViewMode::Tree,
            entity_groups: HashMap::new(),
            inspector_window_entity: None,
        }
    }

    pub fn toggle_group(&mut self, group_id: &str) {
        if self.expanded_groups.contains(group_id) {
            self.expanded_groups.remove(group_id);
        } else {
            self.expanded_groups.insert(group_id.to_string());
        }
    }

    pub fn is_group_expanded(&self, group_id: &str) -> bool {
        self.expanded_groups.contains(group_id)
    }

    pub fn select_entity(&mut self, entity: Entity) {
        self.selected_entity = Some(entity);
        self.view_mode = ViewMode::Detailed;
    }

    pub fn clear_selection(&mut self) {
        self.selected_entity = None;
        self.view_mode = ViewMode::Tree;
    }
}

/// Inspector view modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Hierarchical tree view grouped by components
    #[default]
    Tree,
    /// Flat list view of all entities
    List,
    /// Detailed view of a single entity's components
    Detailed,
}

/// Marker component for the details panel
#[derive(Component)]
pub struct InspectorDetailsPanel;
