//! Component viewer UI with live data updates

use bevy::prelude::*;
use crate::http_client::HttpRemoteClient;
use crate::ui::entity_list::{SelectedEntity, EntityCache};
use serde_json::Value;
use std::collections::HashMap;

/// Component for the component viewer panel
#[derive(Component)]
pub struct ComponentViewerPanel;

/// Component to track component data that needs live updates
#[derive(Component)]
pub struct ComponentData {
    pub entity_id: u32,
    pub component_name: String,
}

/// Resource to cache component data for live updates
#[derive(Resource, Default)]
pub struct ComponentCache {
    pub current_entity: Option<u32>,
    pub components: HashMap<String, Value>,
    pub last_update: f64,
    pub ui_built_for_entity: Option<u32>, // Track which entity we've built UI for
}

/// System to update component viewer when entity selection changes
pub fn update_component_viewer(
    mut commands: Commands,
    _http_client: Res<HttpRemoteClient>,
    entity_cache: Res<EntityCache>,
    mut component_cache: ResMut<ComponentCache>,
    selected_entity: Res<SelectedEntity>,
    time: Res<Time>,
    viewer_query: Query<Entity, With<ComponentViewerPanel>>,
) {
    let Ok(viewer_entity) = viewer_query.single() else {
        println!("❌ Component viewer entity not found");
        return;
    };

    // Check if we need to update (entity changed or periodic refresh)
    let entity_changed = component_cache.current_entity != selected_entity.entity_id;
    let ui_needs_rebuild = component_cache.ui_built_for_entity != selected_entity.entity_id;
    let should_refresh = false; // Disable automatic refresh for now - only update when entity changes

    // Only debug when something interesting happens
    if entity_changed || should_refresh || ui_needs_rebuild {
        println!("🔄 Component viewer update: entity_changed={}, ui_needs_rebuild={}, should_refresh={}, current={:?}, selected={:?}", 
            entity_changed, ui_needs_rebuild, should_refresh, component_cache.current_entity, selected_entity.entity_id);
    }

    if !entity_changed && !should_refresh && !ui_needs_rebuild {
        return;
    }
    
    println!("🔥 REBUILDING component viewer content");
    // Only update timestamp if we're actually going to rebuild
    component_cache.last_update = time.elapsed_secs_f64();

    if entity_changed {
        component_cache.current_entity = selected_entity.entity_id;
        component_cache.components.clear();
    }
    
    // Mark that we're about to build UI for this entity
    if let Some(entity_id) = selected_entity.entity_id {
        component_cache.ui_built_for_entity = Some(entity_id);
    } else {
        component_cache.ui_built_for_entity = None;
    }

    if let Some(entity_id) = selected_entity.entity_id {
        println!("Updating component viewer for entity: {}", entity_id);
        
        // Get component data from entity cache
        let components = if let Some(entity) = entity_cache.entities.get(&entity_id) {
            println!("Found entity {} with {} components", entity_id, entity.components.len());
            &entity.components
        } else {
            println!("Entity {} not found in entity cache", entity_id);
            &HashMap::new()
        };
        
        spawn_component_sections(&mut commands, viewer_entity, entity_id, components);
    } else {
        println!("No entity selected, showing empty state");
        // Show empty state
        spawn_empty_state(&mut commands, viewer_entity);
    }
}

/// Component marker for clearing content
#[derive(Component)]
pub struct ComponentViewerContent {
    pub entity_id: u32,
}

/// Spawn component sections for an entity
fn spawn_component_sections(
    commands: &mut Commands,
    parent: Entity,
    entity_id: u32,
    components: &HashMap<String, Value>,
) {
    println!("🏗️ Building component sections for entity {}", entity_id);
    
    // Use with_children to properly manage the parent-child relationship
    commands.entity(parent).with_children(|parent| {
        // Header
        parent.spawn((
            ComponentViewerContent { entity_id },
            Text::new(format!("Entity {} Components", entity_id)),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 1.0)),
            Node {
                margin: UiRect::bottom(Val::Px(16.0)),
                ..default()
            },
        ));

        // Scrollable content area
        let scroll_container = parent.spawn((
            ComponentViewerContent { entity_id },
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip_y(),
                ..default()
            },
        )).id();

        // Create collapsible sections for each component
        if components.is_empty() {
            // Placeholder components for testing
            let placeholder_components = vec![
                ("Transform", r#"Transform {
    translation: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
    rotation: Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 },
    scale: Vec3 { x: 1.0, y: 1.0, z: 1.0 }
}"#),
                ("Name", r#"Name("Entity Name")"#),
                ("Visibility", r#"Visibility::Inherited"#),
            ];

            for (component_name, component_data) in placeholder_components {
                create_component_section(&mut parent.commands(), scroll_container, entity_id, component_name, component_data);
            }
        } else {
            for (component_name, component_value) in components {
                let formatted_data = format_component_value(component_value);
                create_component_section(&mut parent.commands(), scroll_container, entity_id, component_name, &formatted_data);
            }
        }
    });
}

/// System to clean up old component viewer content
pub fn cleanup_old_component_content(
    mut commands: Commands,
    content_query: Query<(Entity, &ComponentViewerContent)>,
    selected_entity: Res<SelectedEntity>,
) {
    if let Some(current_entity_id) = selected_entity.entity_id {
        for (entity, content) in content_query.iter() {
            if content.entity_id != current_entity_id {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Create a collapsible section for a component
/// Get enhanced display info for a component
fn get_component_display_info(component_name: &str) -> (String, String, String) {
    let short_name = component_name.split("::").last().unwrap_or(component_name);
    
    // Categorize component types for better organization
    let (category, display_name) = if component_name.starts_with("bevy_") {
        // Built-in Bevy components - extract module for category
        let parts: Vec<&str> = component_name.split("::").collect();
        let category = if parts.len() >= 2 {
            match parts[0] {
                "bevy_transform" => "Transform",
                "bevy_render" => "Rendering", 
                "bevy_sprite" => "2D Graphics",
                "bevy_pbr" => "3D Graphics",
                "bevy_ui" => "UI",
                "bevy_text" => "Text",
                "bevy_audio" => "Audio",
                "bevy_input" => "Input",
                "bevy_window" => "Window",
                "bevy_core" => "Core",
                "bevy_hierarchy" => "Hierarchy",
                _ => "Bevy"
            }
        } else {
            "Bevy"
        };
        (category, short_name.to_string())
    } else {
        // Custom components - show as custom
        ("Custom", short_name.to_string())
    };
    
    (category.to_string(), display_name, component_name.to_string())
}

fn create_component_section(
    commands: &mut Commands,
    parent: Entity,
    entity_id: u32,
    component_name: &str,
    component_data: &str,
) {
    let (category, display_name, full_path) = get_component_display_info(component_name);
    
    // Create the section manually to have more control
    let section_entity = commands.spawn((
        crate::ui::CollapsibleSection {
            title: display_name.clone(),
            is_expanded: true,
            header_entity: None,
            content_entity: None,
        },
        Node {
            width: Val::Percent(100.0),
            margin: UiRect::bottom(Val::Px(4.0)),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgb(0.15, 0.15, 0.2)),
        BorderColor::all(Color::srgb(0.3, 0.3, 0.4)),
    )).id();
    
    commands.entity(parent).add_child(section_entity);
    
    // Create header
    let header_entity = commands.spawn((
        Button,
        Node {
            width: Val::Percent(100.0),
            height: if full_path != display_name { Val::Px(48.0) } else { Val::Px(32.0) },
            padding: UiRect::all(Val::Px(8.0)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
        crate::ui::CollapsibleHeader { section_entity },
    )).with_children(|parent| {
        // Component name and category
        parent.spawn((
            Text::new(format!("▼ {} [{}]", display_name, category)),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.6)),
        ));
        
        // Full path in smaller text
        if full_path != display_name {
            parent.spawn((
                Text::new(full_path.clone()),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
                Node {
                    margin: UiRect::top(Val::Px(1.0)),
                    ..default()
                },
            ));
        }
    }).id();
    
    // Create content
    let content_entity = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(8.0)),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
        crate::ui::CollapsibleContent { section_entity },
    )).with_children(|parent| {
        parent.spawn((
            Text::new(component_data),
            TextFont {
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            Node {
                width: Val::Percent(100.0),
                ..default()
            },
            ComponentData {
                entity_id,
                component_name: component_name.to_string(),
            },
        ));
    }).id();
    
    // Link everything together
    commands.entity(section_entity).add_child(header_entity);
    commands.entity(section_entity).add_child(content_entity);
    
    // Update the section with entity references
    commands.entity(section_entity).insert(crate::ui::CollapsibleSection {
        title: display_name.to_string(),
        is_expanded: true,
        header_entity: Some(header_entity),
        content_entity: Some(content_entity),
    });
}

/// Show empty state when no entity is selected
fn spawn_empty_state(commands: &mut Commands, parent: Entity) {
    // For now, don't clear
    
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Text::new("No entity selected\n\nSelect an entity from the list to view its components."),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.6, 0.6, 0.6)),
            Node {
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                ..default()
            },
        ));
    });
}

/// Format a JSON value for display
fn format_component_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        _ => serde_json::to_string_pretty(value).unwrap_or_else(|_| "Invalid JSON".to_string()),
    }
}

/// Spawn the component viewer UI
pub fn spawn_component_viewer(commands: &mut Commands, parent: Entity) -> Entity {
    let viewer = commands.spawn((
        ComponentViewerPanel,
        Node {
            width: Val::Px(800.0), // Fixed width  
            height: Val::Vh(100.0),
            padding: UiRect::all(Val::Px(16.0)),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollPosition::default(),
        BackgroundColor(Color::srgb(0.3, 0.2, 0.2)), // More visible color
    )).id();
    
    commands.entity(parent).add_child(viewer);
    viewer
}