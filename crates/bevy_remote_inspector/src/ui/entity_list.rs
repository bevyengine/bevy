//! Entity list UI component with remote data binding

use bevy::prelude::*;
use crate::http_client::RemoteEntity;
use std::collections::HashMap;

/// Component for the entity list panel
#[derive(Component)]
pub struct EntityListPanel;

/// Component for individual entity list items
#[derive(Component)]
pub struct EntityListItem {
    pub entity_id: u32,
}

/// Resource to track currently selected entity
#[derive(Resource, Default)]
pub struct SelectedEntity {
    pub entity_id: Option<u32>,
}

/// Resource to cache entity data
#[derive(Resource, Default)]
pub struct EntityCache {
    pub entities: HashMap<u32, RemoteEntity>,
    pub last_update: f64,
}

/// Marker component for the scrollable list container
#[derive(Component)]
pub struct EntityListContainer;


/// System to handle entity selection
pub fn handle_entity_selection(
    mut selected_entity: ResMut<SelectedEntity>,
    interaction_query: Query<(&Interaction, &EntityListItem), Changed<Interaction>>,
    all_buttons: Query<Entity, With<EntityListItem>>,
    time: Res<Time>,
) {
    // Only show debug info occasionally
    if (time.elapsed_secs() as i32) % 30 == 0 && time.delta_secs() < 0.1 {
        println!("Entity selection system: {} buttons available", all_buttons.iter().count());
    }
    
    // Only process actual clicks, not hover events
    for (interaction, item) in interaction_query.iter() {
        match *interaction {
            Interaction::Pressed => {
                selected_entity.entity_id = Some(item.entity_id);
                println!("Selected entity: {}", item.entity_id);
            }
            // Don't spam logs for None/Hovered states
            _ => {}
        }
    }
}

/// Simple scrolling state for entity list (simplified for now)
#[derive(Resource, Default)]
pub struct EntityListVirtualState {
    pub item_height: f32,
}

impl EntityListVirtualState {
    pub fn new() -> Self {
        Self {
            item_height: 34.0, // 32px height + 2px margin
        }
    }
}

/// Spawn the entity list UI with virtual scrolling
pub fn spawn_entity_list(commands: &mut Commands, parent: Entity) -> Entity {
    let panel = commands.spawn((
        EntityListPanel,
        Node {
            width: Val::Px(360.0), // Fixed width instead of percentage
            height: Val::Vh(100.0),
            padding: UiRect::all(Val::Px(8.0)),
            flex_direction: FlexDirection::Column,
            border: UiRect::right(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.2, 0.3)), // More visible color
        BorderColor::all(Color::srgb(0.5, 0.5, 0.6)),
    )).with_children(|parent| {
        // Header
        parent.spawn((
            Text::new("Entities"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::bottom(Val::Px(12.0)),
                ..default()
            },
        ));
        
        // Scrollable container with mouse wheel support
        parent.spawn((
            EntityListContainer,
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
        )).with_children(|parent| {
            // Virtual scrolling content - this will be dynamically populated
            parent.spawn((
                EntityListVirtualContent,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    // Remove min_height - let virtual scrolling control the size
                    ..default()
                },
            ));
        });
    }).id();
    
    commands.entity(parent).add_child(panel);
    panel
}

/// Marker for the virtual scrolling content area
#[derive(Component)]
pub struct EntityListVirtualContent;

/// Determine the best display name for an entity based on its components
fn get_entity_display_info(entity: &RemoteEntity) -> (String, String) {
    let entity_id = entity.id.to_string();
    
    // Priority 1: Use Name component if available
    if let Some(name) = &entity.name {
        return (entity_id, name.clone());
    }
    
    // Priority 2: Look for custom components (non-bevy components)
    let custom_components: Vec<&String> = entity.components.keys()
        .filter(|name| !name.starts_with("bevy_") && !name.starts_with("std::"))
        .collect();
    
    if !custom_components.is_empty() {
        // Prioritize specific known custom components from our target app
        let priority_custom = ["Player", "Enemy", "Item"];
        
        for priority_name in &priority_custom {
            if let Some(component_name) = custom_components.iter()
                .find(|name| name.contains(priority_name)) {
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
    
    // Priority 3: Look for common recognizable bevy components
    let common_components = [
        ("bevy_render::camera::camera::Camera", "Camera"),
        ("bevy_sprite::sprite::Sprite", "Sprite"),
        ("bevy_text::text::Text", "Text"),
        ("bevy_ui::node::Node", "UI Node"),
        ("bevy_pbr::pbr_material::StandardMaterial", "Material"),
        ("bevy_render::mesh::mesh::Mesh3d", "Mesh"),
        ("bevy_light::directional_light::DirectionalLight", "Light"),
        ("bevy_light::point_light::PointLight", "Point Light"),
        ("bevy_window::window::Window", "Window"),
    ];
    
    for (full_name, display_name) in &common_components {
        if entity.components.contains_key(*full_name) {
            return (entity_id, display_name.to_string());
        }
    }
    
    // Priority 4: Use Transform if present (very common)
    if entity.components.contains_key("bevy_transform::components::transform::Transform") {
        return (entity_id, "Transform".to_string());
    }
    
    // Fallback: Just show Entity
    (entity_id, "Entity".to_string())
}

/// Spawn an entity list item
pub fn spawn_entity_list_item(
    commands: &mut Commands,
    parent: Entity,
    entity: &RemoteEntity,
) -> Entity {
    let (entity_id, description) = get_entity_display_info(entity);
    let display_text = format!("{} ({})", entity_id, description);
    
    // Create button with explicit interaction setup
    let item = commands.spawn((
        EntityListItem { entity_id: entity.id },
        Button,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(32.0), // Slightly taller for better click target
            margin: UiRect::bottom(Val::Px(2.0)),
            padding: UiRect::all(Val::Px(8.0)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::FlexStart,
            ..default()
        },
        BackgroundColor(Color::srgb(0.3, 0.3, 0.4)), // More distinct color
        BorderColor::all(Color::srgb(0.5, 0.5, 0.6)),
        // Ensure interaction is properly set up
        Interaction::None,
    )).with_children(|parent| {
        parent.spawn((
            Text::new(display_text),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    }).id();
    
    commands.entity(parent).add_child(item);
    item
}