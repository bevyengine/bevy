//! Entity list UI component with remote data binding

use crate::inspector::http_client::RemoteEntity;
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_text::{TextColor, TextFont};
use bevy_time::Time;
use bevy_ui::prelude::*;
use std::collections::HashMap;

/// Component for the entity list panel
#[derive(Component)]
pub struct EntityListPanel;

/// Component for individual entity list items
#[derive(Component)]
pub struct EntityListItem {
    /// The entity ID this list item represents
    pub entity_id: u32,
}

/// Resource to track currently selected entity
#[derive(Resource, Default)]
pub struct SelectedEntity {
    /// ID of the currently selected entity
    pub entity_id: Option<u32>,
}

/// Resource to cache entity data
#[derive(Resource, Default)]
pub struct EntityCache {
    /// Map of entity IDs to their cached data
    pub entities: HashMap<u32, RemoteEntity>,
    /// Timestamp of the last cache update
    pub last_update: f64,
}

/// Marker component for the scrollable list container
#[derive(Component)]
pub struct EntityListContainer;

/// Resource to track last selection time to prevent flashing
#[derive(Resource, Default)]
pub struct SelectionDebounce {
    /// Timestamp of the last entity selection
    pub last_selection_time: f64,
    /// Minimum time between selections in seconds
    pub debounce_interval: f64,
    /// Timestamp of the last virtual scroll update
    pub last_virtual_scroll_time: f64,
    /// Duration to prevent interactions after scroll in seconds
    pub scroll_interaction_lockout: f64,
}

impl SelectionDebounce {
    /// Create a new selection debounce tracker with default settings
    pub fn new() -> Self {
        Self {
            last_selection_time: 0.0,
            debounce_interval: 0.05, // 50ms debounce to prevent rapid flashing
            last_virtual_scroll_time: 0.0,
            scroll_interaction_lockout: 0.1, // 100ms lockout after virtual scrolling (reduced)
        }
    }
}

/// System to handle entity selection
pub fn handle_entity_selection(
    mut selected_entity: ResMut<SelectedEntity>,
    mut selection_debounce: ResMut<SelectionDebounce>,
    interaction_query: Query<(&Interaction, &EntityListItem), Changed<Interaction>>,
    _all_buttons: Query<Entity, With<EntityListItem>>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs_f64();

    // Only process actual clicks, not hover events, and prevent duplicate selections
    for (interaction, item) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            // Debounce rapid selections to prevent flashing during virtual scrolling
            if current_time - selection_debounce.last_selection_time
                < selection_debounce.debounce_interval
            {
                continue;
            }

            // Block interactions shortly after virtual scrolling updates
            if current_time - selection_debounce.last_virtual_scroll_time
                < selection_debounce.scroll_interaction_lockout
            {
                continue;
            }

            // Only update if it's actually a different entity
            if selected_entity.entity_id != Some(item.entity_id) {
                selected_entity.entity_id = Some(item.entity_id);
                selection_debounce.last_selection_time = current_time;
            }
        }
    }
}

/// Simple scrolling state for entity list (simplified for now)
#[derive(Resource, Default)]
pub struct EntityListVirtualState {
    /// Height of each entity list item in pixels
    pub item_height: f32,
}

impl EntityListVirtualState {
    /// Create a new virtual scrolling state with default settings
    pub fn new() -> Self {
        Self {
            item_height: 34.0, // 34px height with no margin
        }
    }
}

/// Spawn the entity list UI with virtual scrolling and scrollbar
pub fn spawn_entity_list(commands: &mut Commands, parent: Entity) -> Entity {
    let panel = commands
        .spawn((
            EntityListPanel,
            Node {
                width: Val::Px(360.0), // Fixed width instead of percentage
                height: Val::Vh(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                border: UiRect::right(Val::Px(1.0)),
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.3)), // More visible color
            BorderColor::all(Color::srgb(0.5, 0.5, 0.6)),
        ))
        .with_children(|parent| {
            // Header
            parent.spawn((
                Text::new("Entities"),
                TextFont {
                    font_size: 18.0,
                    ..Default::default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(12.0)),
                    ..Default::default()
                },
            ));

            // Outer viewport container - no scrollbars since we use custom scroll
            parent
                .spawn((
                    EntityListContainer,
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        position_type: PositionType::Relative,
                        overflow: Overflow::hidden(), // Hidden - we handle scroll ourselves
                        ..Default::default()
                    },
                    ScrollPosition::default(),
                    BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
                ))
                .with_children(|parent| {
                    // Inner content container - viewport height, items positioned relative to scroll
                    parent.spawn((
                        EntityListVirtualContent,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0), // Fill viewport
                            position_type: PositionType::Relative,
                            overflow: Overflow::hidden(),
                            ..Default::default()
                        },
                    ));
                });

            // Add a visual scrollbar indicator positioned absolutely (doesn't affect layout)
            parent
                .spawn((
                    ScrollbarIndicator,
                    Node {
                        width: Val::Px(8.0),
                        height: Val::Percent(95.0), // Slightly shorter than container
                        position_type: PositionType::Absolute,
                        right: Val::Px(4.0),
                        top: Val::Percent(2.5), // Center vertically
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.15)), // Track color
                ))
                .with_children(|parent| {
                    // Scrollbar thumb
                    parent.spawn((
                        ScrollbarThumb,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(50.0), // Will be updated dynamically
                            position_type: PositionType::Absolute,
                            top: Val::Px(0.0), // Will be updated dynamically
                            ..Default::default()
                        },
                        BackgroundColor(Color::srgb(0.4, 0.4, 0.5)), // Thumb color
                    ));
                });
        })
        .id();

    commands.entity(parent).add_child(panel);
    panel
}

/// Marker for the virtual scrolling content area
#[derive(Component)]
pub struct EntityListVirtualContent;

/// Component for the scrollbar indicator
#[derive(Component)]
pub struct ScrollbarIndicator;

/// Component for the scrollbar thumb
#[derive(Component)]
pub struct ScrollbarThumb;

/// Determine the best display name for an entity based on its components
fn get_entity_display_info(entity: &RemoteEntity) -> (String, String) {
    let entity_id = entity.id.to_string();

    // Priority 1: Use Name component if available
    if let Some(name) = &entity.name {
        return (entity_id, name.clone());
    }

    // Priority 2: Look for custom components (non-bevy components)
    let custom_components: Vec<&String> = entity
        .components
        .keys()
        .filter(|name| !name.starts_with("bevy_") && !name.starts_with("std::"))
        .collect();

    if !custom_components.is_empty() {
        // Prioritize specific known custom components from our target app
        let priority_custom = ["Player", "Enemy", "Item"];

        for priority_name in &priority_custom {
            if let Some(component_name) = custom_components
                .iter()
                .find(|name| name.contains(priority_name))
            {
                let display_name = component_name
                    .split("::")
                    .last()
                    .unwrap_or(component_name)
                    .to_string();
                return (entity_id, display_name);
            }
        }

        // Use the first custom component if no priority match
        let component_name = custom_components[0];
        let display_name = component_name
            .split("::")
            .last()
            .unwrap_or(component_name)
            .to_string();
        return (entity_id, display_name);
    }

    // Priority 3: Look for high-priority recognizable bevy components
    let high_priority_components = [
        ("bevy_render::camera::camera::Camera", "Camera"),
        ("bevy_sprite::sprite::Sprite", "Sprite"),
        ("bevy_text::text::Text", "Text"),
        ("bevy_ui::node::Node", "UI Node"),
        ("bevy_window::window::Window", "Window"),
        ("bevy_pbr::pbr_material::StandardMaterial", "Material"),
        ("bevy_render::mesh::mesh::Mesh3d", "Mesh3D"),
        ("bevy_render::mesh::mesh::Mesh2d", "Mesh2D"),
        (
            "bevy_light::directional_light::DirectionalLight",
            "Dir Light",
        ),
        ("bevy_light::point_light::PointLight", "Point Light"),
        ("bevy_light::spot_light::SpotLight", "Spot Light"),
        ("bevy_audio::audio_source::AudioSource", "Audio"),
    ];

    for (full_name, display_name) in &high_priority_components {
        if entity.components.contains_key(*full_name) {
            return (entity_id, display_name.to_string());
        }
    }

    // Priority 4: Look for other meaningful bevy components
    let secondary_components = [
        (
            "bevy_transform::components::global_transform::GlobalTransform",
            "GlobalTransform",
        ),
        (
            "bevy_transform::components::transform::Transform",
            "Transform",
        ),
        ("bevy_render::view::visibility::Visibility", "Visible"),
        (
            "bevy_render::view::visibility::InheritedVisibility",
            "Inherited",
        ),
        (
            "bevy_render::view::visibility::ViewVisibility",
            "ViewVisible",
        ),
        ("bevy_hierarchy::components::parent::Parent", "Child"),
        ("bevy_hierarchy::components::children::Children", "Parent"),
        ("bevy_asset::handle::Handle", "Asset"),
    ];

    for (full_name, display_name) in &secondary_components {
        if entity.components.contains_key(*full_name) {
            return (entity_id, display_name.to_string());
        }
    }

    // Priority 5: Show multiple component types if available
    let mut component_types = Vec::new();

    // Count different types of components
    let has_transform = entity.components.keys().any(|k| k.contains("Transform"));
    let has_render = entity
        .components
        .keys()
        .any(|k| k.contains("render") || k.contains("Mesh") || k.contains("Material"));
    let has_ui = entity
        .components
        .keys()
        .any(|k| k.contains("ui") || k.contains("Node"));
    let has_audio = entity
        .components
        .keys()
        .any(|k| k.contains("audio") || k.contains("Audio"));
    let has_hierarchy = entity
        .components
        .keys()
        .any(|k| k.contains("Parent") || k.contains("Children"));

    if has_render {
        component_types.push("Render");
    }
    if has_ui {
        component_types.push("UI");
    }
    if has_audio {
        component_types.push("Audio");
    }
    if has_hierarchy {
        component_types.push("Parent/Child");
    }
    if has_transform {
        component_types.push("Transform");
    }

    if !component_types.is_empty() {
        let display_name = if component_types.len() == 1 {
            component_types[0].to_string()
        } else {
            format!("{} (+{})", component_types[0], component_types.len() - 1)
        };
        return (entity_id, display_name);
    }

    // Fallback: Show component count
    let component_count = entity.components.len();
    if component_count > 0 {
        (entity_id, format!("Entity ({component_count}c)"))
    } else {
        (entity_id, "Empty Entity".to_string())
    }
}

/// Spawn an entity list item
pub fn spawn_entity_list_item(
    commands: &mut Commands,
    parent: Entity,
    entity: &RemoteEntity,
) -> Entity {
    let (entity_id, description) = get_entity_display_info(entity);
    let display_text = format!("{entity_id} ({description})");

    // Create button with explicit interaction setup
    let item = commands
        .spawn((
            EntityListItem {
                entity_id: entity.id,
            },
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(34.0), // Match virtual scrolling item_height exactly
                margin: UiRect::ZERO,  // Remove margin to eliminate gaps
                padding: UiRect::all(Val::Px(8.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.3, 0.3, 0.4)), // More distinct color
            BorderColor::all(Color::srgb(0.5, 0.5, 0.6)),
            // Ensure interaction is properly set up
            Interaction::None,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(display_text),
                TextFont {
                    font_size: 12.0,
                    ..Default::default()
                },
                TextColor(Color::WHITE),
            ));
        })
        .id();

    commands.entity(parent).add_child(item);
    item
}
