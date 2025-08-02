//! Core systems for the entity inspector.

use bevy_camera::{Camera, Camera2d};
use bevy_ecs::{
    entity::Entity,
    name::Name,
    query::{With, Without},
    system::{Commands, Query, Res, ResMut, Local},
    world::World,
};
use bevy_input::ButtonInput;
use bevy_ecs::prelude::{AppTypeRegistry, ReflectComponent};
use bevy_render::camera::RenderTarget;
use bevy_state::prelude::*;
use bevy_ui::{BackgroundColor, BorderColor, FlexDirection, Interaction, Node, Val};
use bevy_window::{Window, WindowRef};

use super::{ui, InspectorConfig, InspectorData, InspectorState};


/// Helper function to format component data for display with proper truncation and formatting.
fn format_component_data(value_str: &str, max_line_length: usize, max_lines: usize) -> String {
    // Handle very long single-line values (common with Bevy's debug output)
    if value_str.len() > max_line_length * 3 && !value_str.contains('\n') {
        // For very long single lines, just truncate with context
        return format!("{}...\n(truncated, {} total chars)", 
                      &value_str[..max_line_length.min(value_str.len())], 
                      value_str.len());
    }
    
    // Immediately truncate if the total content is too large
    let mut working_str = if value_str.len() > 1000 {
        format!("{}...\n(truncated from {} chars)", &value_str[..800], value_str.len())
    } else {
        value_str.to_string()
    };
    
    // Try to make the debug output more readable by adding strategic line breaks
    // Handle struct-like patterns
    if working_str.contains(" { ") {
        working_str = working_str
            .replace(" { ", " {\n  ")
            .replace(", ", ",\n  ")
            .replace(" }", "\n}");
    }
    
    // Handle array/vec patterns  
    if working_str.contains(": [") {
        working_str = working_str
            .replace(": [", ": [\n    ")
            .replace(", ", ",\n    ")
            .replace(" ]", "\n  ]");
    }
    
    let lines: Vec<&str> = working_str.lines().collect();
    let mut result_lines = Vec::new();
    
    for (i, line) in lines.iter().enumerate() {
        if i >= max_lines {
            result_lines.push(format!("  ... ({} more lines)", lines.len() - max_lines));
            break;
        }
        
        if line.len() > max_line_length {
            let truncated = format!("{}...", &line[..max_line_length.min(line.len())]);
            result_lines.push(truncated);
        } else {
            result_lines.push(line.to_string());
        }
    }
    
    result_lines.join("\n")
}

/// System that handles input for toggling the inspector window.
pub fn handle_toggle_input(
    keyboard_input: Res<ButtonInput<bevy_input::keyboard::KeyCode>>,
    config: Res<InspectorConfig>,
    current_state: Res<State<InspectorState>>,
    mut next_state: ResMut<NextState<InspectorState>>,
) {
    if keyboard_input.just_pressed(config.toggle_key) {
        match current_state.get() {
            InspectorState::Inactive => {
                next_state.set(InspectorState::Active);
            }
            InspectorState::Active => {
                next_state.set(InspectorState::Inactive);
            }
        }
    }
}

/// System that manages the inspector window lifecycle.
pub fn manage_inspector_window(
    mut commands: Commands,
    mut inspector_data: ResMut<InspectorData>,
    current_state: Res<State<InspectorState>>,
    config: Res<InspectorConfig>,
) {
    match current_state.get() {
        InspectorState::Active => {
            if inspector_data.ui_root.is_none() {
                if config.use_overlay_mode {
                    // Overlay mode: render directly on main window (no separate window or camera)
                    bevy_log::info!("Creating inspector overlay mode");
                    let ui_root = ui::create_inspector_overlay(&mut commands);
                    inspector_data.ui_root = Some(ui_root);
                    // Set camera to None to indicate overlay mode
                    inspector_data.inspector_camera = None;
                    inspector_data.inspector_window = None;
                    bevy_log::info!("Inspector overlay created with UI root: {:?}", ui_root);
                } else {
                    // Separate window mode: create new window and camera
                    let window_entity = commands
                        .spawn((
                            Window {
                                title: "Bevy Entity Inspector".to_string(),
                                resolution: (800.0, 600.0).into(),
                                ..Default::default()
                            },
                            InspectorEntity,
                        ))
                        .id();

                    let camera_entity = commands
                        .spawn((
                            Camera2d,
                            Camera {
                                target: RenderTarget::Window(WindowRef::Entity(window_entity)),
                                ..Default::default()
                            },
                            InspectorEntity,
                        ))
                        .id();

                    inspector_data.inspector_window = Some(window_entity);
                    inspector_data.inspector_camera = Some(camera_entity);

                    let ui_root = ui::create_inspector_ui(&mut commands, camera_entity);
                    inspector_data.ui_root = Some(ui_root);
                }
            }
        }
        InspectorState::Inactive => {
            if let Some(window_entity) = inspector_data.inspector_window.take() {
                if let Ok(mut entity_commands) = commands.get_entity(window_entity) {
                    entity_commands.despawn();
                }
            }
            if let Some(camera_entity) = inspector_data.inspector_camera.take() {
                if let Ok(mut entity_commands) = commands.get_entity(camera_entity) {
                    entity_commands.despawn();
                }
            }
            if let Some(ui_root) = inspector_data.ui_root.take() {
                if let Ok(mut entity_commands) = commands.get_entity(ui_root) {
                    entity_commands.despawn();
                }
            }
            inspector_data.selected_entity = None;
        }
    }
}

/// Marker component for the entity list container.
#[derive(bevy_ecs::component::Component)]
pub struct EntityListContainer;

/// Marker component for entity list buttons.
#[derive(bevy_ecs::component::Component)]
pub struct EntityListButton {
    pub entity: Entity,
}

/// Marker component for the component viewer container.
#[derive(bevy_ecs::component::Component)]
pub struct ComponentViewerContainer;

/// Marker component for all entities created by the inspector.
/// This helps us avoid including inspector UI entities in the entity list.
#[derive(bevy_ecs::component::Component)]
pub struct InspectorEntity;

/// Marker component for component value text that needs live updates.
#[derive(bevy_ecs::component::Component)]
pub struct ComponentValueText {
    pub entity: Entity,
    pub component_name: String,
}

/// Component to track collapsible sections
#[derive(bevy_ecs::component::Component)]
pub struct CollapsibleSection {
    pub component_name: String,
    pub is_expanded: bool,
    pub content_entity: Option<Entity>,
}

/// Resource to track inspector state to prevent infinite loops
#[derive(bevy_ecs::prelude::Resource)]
struct InspectorLastState {
    last_selected_entity: Option<Entity>,
    last_rebuild_time: f32,
    last_update_time: f32,
}

/// System that populates the entity list in the inspector.
pub fn populate_entity_list(
    mut commands: Commands,
    _inspector_data: Res<InspectorData>,
    current_state: Res<State<InspectorState>>,
    all_entities: Query<Entity, Without<InspectorEntity>>,
    entity_names: Query<&Name>,
    list_container: Query<Entity, With<EntityListContainer>>,
    existing_buttons: Query<Entity, With<EntityListButton>>,
    mut last_entity_count: Local<usize>,
    mut update_cooldown: Local<f32>,
    time: Res<bevy_time::Time>,
) {
    // Only run when inspector is active
    if *current_state.get() != InspectorState::Active {
        return;
    }

    // Update cooldown timer
    *update_cooldown -= time.delta_secs();

    // Only run if we have a list container
    let Ok(container_entity) = list_container.single() else {
        bevy_log::debug!("No EntityListContainer found, entities in query: {}", list_container.iter().count());
        return;
    };
    
    bevy_log::debug!("Found EntityListContainer: {:?}", container_entity);

    let entity_count = all_entities.iter().count();
    let existing_buttons_count = existing_buttons.iter().count();
    
    // Only update if enough time has passed AND the entity count changed
    // OR if this is the first time we've found the container (buttons_count == 0)
    let count_changed = *last_entity_count != entity_count;
    let cooldown_ready = *update_cooldown <= 0.0;
    let first_population = existing_buttons_count == 0 && entity_count > 0;
    
    bevy_log::debug!("Entity list check: {} entities, {} existing buttons, count_changed: {}, cooldown_ready: {}, first_population: {}", 
                     entity_count, existing_buttons_count, count_changed, cooldown_ready, first_population);

    if (!count_changed || !cooldown_ready) && !first_population {
        return;
    }

    // Reset cooldown (only update once every 5 seconds for better performance)
    *update_cooldown = 5.0;
    *last_entity_count = entity_count;

    bevy_log::info!("Updating entity list: {} entities (was {})", entity_count, existing_buttons_count);

    // Clear existing buttons
    for button_entity in existing_buttons.iter() {
        if let Ok(mut entity_commands) = commands.get_entity(button_entity) {
            entity_commands.despawn();
        }
    }

    // Add entities to the list
    commands.entity(container_entity).with_children(|parent| {
        for entity in all_entities.iter() {

            let entity_name = entity_names
                .get(entity)
                .map(|name| name.as_str())
                .unwrap_or("Unnamed");

            let display_text = format!("{} ({})", entity_name, entity.index());

            parent
                .spawn((
                    bevy_ui::widget::Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(24.0),
                        margin: bevy_ui::UiRect::bottom(Val::Px(2.0)),
                        padding: bevy_ui::UiRect::all(Val::Px(4.0)),
                        ..Default::default()
                    },
                    BackgroundColor(bevy_color::Color::srgb(0.2, 0.2, 0.2)),
                    BorderColor::all(bevy_color::Color::srgb(0.4, 0.4, 0.4)),
                    EntityListButton { entity },
                    InspectorEntity,
                ))
                .with_child((
                    bevy_ui::widget::Text::new(display_text),
                    bevy_text::TextFont {
                        font_size: 12.0,
                        ..Default::default()
                    },
                    bevy_text::TextColor(bevy_color::Color::WHITE),
                    InspectorEntity,
                ));
        }
    });
}

/// System that handles clicking on entity list buttons.
pub fn handle_entity_selection(
    mut inspector_data: ResMut<InspectorData>,
    mut interaction_query: Query<(&Interaction, &EntityListButton), bevy_ecs::query::Changed<Interaction>>,
) {
    for (interaction, entity_button) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            inspector_data.selected_entity = Some(entity_button.entity);
        }
    }
}

/// System that processes all inspector updates with exclusive World access.
/// This runs in the InspectorLast schedule to avoid World borrow conflicts,
/// following the same pattern as bevy_remote.
pub fn process_inspector_updates(world: &mut World) {
    // Check if inspector is active
    let current_state = world.resource::<State<InspectorState>>();
    if *current_state.get() != InspectorState::Active {
        return;
    }
    
    // Get the selected entity from inspector data
    let inspector_data = world.resource::<InspectorData>();
    let selected_entity = inspector_data.selected_entity;
    
    // Check for component viewer container
    let mut component_viewer_query = world.query_filtered::<Entity, With<ComponentViewerContainer>>();
    let Ok(viewer_entity) = component_viewer_query.single(world) else {
        bevy_log::debug!("No ComponentViewerContainer found");
        return;
    };
    
    // Use resource_scope to access local state while avoiding borrow conflicts
    world.resource_scope::<bevy_time::Time, ()>(|world, time| {
        process_component_display_with_world_access(world, viewer_entity, selected_entity, time.delta_secs());
    });
}

/// Helper function that handles component display with exclusive world access.
fn process_component_display_with_world_access(
    world: &mut World, 
    viewer_entity: Entity, 
    selected_entity: Option<Entity>,
    delta_time: f32,
) {
    // Create a persistent resource to track the last state
    if !world.contains_resource::<InspectorLastState>() {
        world.insert_resource(InspectorLastState {
            last_selected_entity: None,
            last_rebuild_time: 0.0,
            last_update_time: 0.0,
        });
    }
    
    let mut last_state = world.resource_mut::<InspectorLastState>();
    
    // Check if entity selection changed
    let entity_changed = last_state.last_selected_entity != selected_entity;
    
    if entity_changed {
        // Always rebuild immediately when entity selection changes
        last_state.last_selected_entity = selected_entity;
        last_state.last_rebuild_time += delta_time;
        drop(last_state); // Release the resource before rebuilding
        
        bevy_log::info!("Entity selection changed to: {:?}, rebuilding immediately", selected_entity);
        rebuild_component_viewer(world, viewer_entity, selected_entity);
    } else if selected_entity.is_some() {
        // For the same entity, rate limit live updates to prevent excessive rebuilds
        let update_cooldown = 0.2; // 200ms between updates (5 FPS)
        let current_time = last_state.last_update_time + delta_time;
        let can_update = current_time - last_state.last_update_time >= update_cooldown;
        
        if can_update {
            last_state.last_update_time = current_time;
            drop(last_state);
            update_live_component_values(world, selected_entity);
        }
    }
}

/// Rebuild the component viewer UI structure.
fn rebuild_component_viewer(world: &mut World, viewer_entity: Entity, selected_entity: Option<Entity>) {
    bevy_log::info!("Rebuilding component viewer for entity: {:?}", selected_entity);
    
    // Clear existing content
    let mut entity_commands = world.entity_mut(viewer_entity);
    entity_commands.clear_children();
    
    if let Some(selected_entity) = selected_entity {
        // Build component list
        build_component_list_exclusive(world, viewer_entity, selected_entity);
    } else {
        // Show empty state
        build_empty_state(world, viewer_entity);
    }
}

/// Build component list with exclusive world access.
fn build_component_list_exclusive(world: &mut World, viewer_entity: Entity, selected_entity: Entity) {
    let Ok(entity_ref) = world.get_entity(selected_entity) else {
        bevy_log::warn!("Selected entity {:?} no longer exists", selected_entity);
        return;
    };
    
    // Get archetype and component info first, before any mutable borrows
    let archetype = entity_ref.archetype();
    let components_info: Vec<_> = archetype.components()
        .filter_map(|component_id| world.components().get_info(component_id))
        .map(|info| (info.name().to_string(), info.name().to_string().split("::").last().unwrap_or("Unknown").to_string()))
        .collect();
    
    // Get reflection data while we still have immutable access
    let app_type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = app_type_registry.read();
    let mut component_data: Vec<(String, String, String)> = Vec::new();
    
    for (component_name_str, short_name) in &components_info {
        if let Some(type_registration) = type_registry.get_with_type_path(component_name_str) {
            if let Some(reflect_component) = type_registration.data::<ReflectComponent>() {
                if let Some(reflected) = reflect_component.reflect(entity_ref) {
                    let data = format!("{:#?}", reflected);
                    component_data.push((component_name_str.clone(), short_name.clone(), data));
                } else {
                    component_data.push((component_name_str.clone(), short_name.clone(), "<not reflectable>".to_string()));
                }
            } else {
                component_data.push((component_name_str.clone(), short_name.clone(), "<no reflection data>".to_string()));
            }
        } else {
            component_data.push((component_name_str.clone(), short_name.clone(), "<not registered>".to_string()));
        }
    }
    
    // Drop the type registry guard before mutable operations
    drop(type_registry);
    
    // Now spawn the component container structure with mutable access
    let mut entity_commands = world.entity_mut(viewer_entity);
    entity_commands.with_children(|parent| {
        // Create a scrollable container for all components
        let mut component_container = parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                overflow: bevy_ui::Overflow::clip_y(),
                ..Default::default()
            },
            InspectorEntity,
        ));
        
        component_container.with_children(|component_parent| {
            // Entity header
            component_parent.spawn((
                bevy_ui::widget::Text::new(format!("Entity {} Components:", selected_entity.index())),
                bevy_text::TextFont {
                    font_size: 16.0,
                    ..Default::default()
                },
                bevy_text::TextColor(bevy_color::Color::srgb(0.9, 0.9, 1.0)),
                Node {
                    margin: bevy_ui::UiRect::bottom(Val::Px(10.0)),
                    ..Default::default()
                },
                InspectorEntity,
            ));

            // Create component sections from pre-collected data
            for (component_name_str, short_name, data) in component_data {
                // Create collapsible component section
                let mut section = component_parent.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        margin: bevy_ui::UiRect::bottom(Val::Px(4.0)),
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    BackgroundColor(bevy_color::Color::srgb(0.15, 0.15, 0.2)),
                    BorderColor::all(bevy_color::Color::srgb(0.3, 0.3, 0.4)),
                    InspectorEntity,
                ));
                
                section.with_children(|section_parent| {
                    // Clickable header with expand/collapse button
                    let mut header_button = section_parent.spawn((
                        bevy_ui::widget::Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(28.0),
                            padding: bevy_ui::UiRect::all(Val::Px(8.0)),
                            justify_content: bevy_ui::JustifyContent::SpaceBetween,
                            align_items: bevy_ui::AlignItems::Center,
                            flex_direction: FlexDirection::Row,
                            ..Default::default()
                        },
                        BackgroundColor(bevy_color::Color::srgb(0.2, 0.2, 0.25)),
                        CollapsibleSection {
                            component_name: component_name_str.clone(),
                            is_expanded: true,
                            content_entity: None,
                        },
                        InspectorEntity,
                    ));
                    
                    header_button.with_children(|header_parent| {
                        // Component name
                        header_parent.spawn((
                            bevy_ui::widget::Text::new(format!("▼ {}", short_name)),
                            bevy_text::TextFont {
                                font_size: 14.0,
                                ..Default::default()
                            },
                            bevy_text::TextColor(bevy_color::Color::srgb(0.9, 0.9, 0.6)),
                            InspectorEntity,
                        ));
                    });
                    
                    // Component content container (initially visible)
                    let mut content_container = section_parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            padding: bevy_ui::UiRect::all(Val::Px(8.0)),
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        BackgroundColor(bevy_color::Color::srgb(0.1, 0.1, 0.15)),
                        InspectorEntity,
                    ));
                    
                    // Add component data
                    content_container.with_children(|content_parent| {
                        let color = if data.starts_with('<') {
                            bevy_color::Color::srgb(0.6, 0.6, 0.6)
                        } else {
                            bevy_color::Color::srgb(0.8, 0.8, 0.8)
                        };
                        
                        content_parent.spawn((
                            bevy_ui::widget::Text::new(data),
                            bevy_text::TextFont {
                                font_size: 10.0,
                                ..Default::default()
                            },
                            bevy_text::TextColor(color),
                            Node {
                                width: Val::Percent(100.0),
                                ..Default::default()
                            },
                            ComponentValueText {
                                entity: selected_entity,
                                component_name: component_name_str,
                            },
                            InspectorEntity,
                        ));
                    });
                });
            }
        });
    });
}

/// Build empty state UI.
fn build_empty_state(world: &mut World, viewer_entity: Entity) {
    let mut entity_commands = world.entity_mut(viewer_entity);
    entity_commands.with_children(|parent| {
        parent.spawn((
            bevy_ui::widget::Text::new("No entity selected\n\nClick on an entity in the left pane to inspect its components."),
            bevy_text::TextFont {
                font_size: 14.0,
                ..Default::default()
            },
            bevy_text::TextColor(bevy_color::Color::srgb(0.6, 0.6, 0.6)),
            InspectorEntity,
        ));
    });
}

/// Update live component values using reflection.
fn update_live_component_values(world: &mut World, selected_entity: Option<Entity>) {
    let Some(selected_entity) = selected_entity else {
        return;
    };
    
    // First, collect all the component value text entities and their info
    let mut text_query = world.query::<(Entity, &bevy_ui::widget::Text, &ComponentValueText)>();
    let mut component_texts: Vec<(Entity, String, String)> = Vec::new();
    
    for (entity_id, text, component_value_text) in text_query.iter(world) {
        // Only collect components for the currently selected entity
        if component_value_text.entity != selected_entity {
            continue;
        }
        
        component_texts.push((entity_id, text.0.clone(), component_value_text.component_name.clone()));
    }
    
    // Now get fresh reflection data for the selected entity
    let Ok(entity_ref) = world.get_entity(selected_entity) else {
        return;
    };
    
    let app_type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = app_type_registry.read();
    
    let mut updates = Vec::new();
    
    for (entity_id, current_text, component_name) in component_texts {
        // Use reflection to get the latest component data
        if let Some(type_registration) = type_registry.get_with_type_path(&component_name) {
            if let Some(reflect_component) = type_registration.data::<ReflectComponent>() {
                if let Some(reflected) = reflect_component.reflect(entity_ref) {
                    let new_component_data = format!("{:#?}", reflected);
                    
                    // Only update if the value has changed
                    if current_text != new_component_data {
                        updates.push((entity_id, new_component_data));
                    }
                }
            }
        }
    }
    
    // Drop the type registry guard before mutable operations
    drop(type_registry);
    
    // Apply updates
    for (entity, new_data) in updates {
        if let Some(mut text) = world.get_mut::<bevy_ui::widget::Text>(entity) {
            text.0 = new_data;
        }
    }
}

/// Legacy system kept for compatibility - now integrated into process_inspector_updates.
/// This will be removed in a future update.
pub fn update_component_values_live() {
    // This system is now integrated into process_inspector_updates
    // to avoid World borrow conflicts
}

/// System that handles clicking on collapsible section headers.
pub fn handle_collapsible_sections(
    mut commands: Commands,
    mut collapsible_query: Query<(&Interaction, &mut CollapsibleSection), bevy_ecs::query::Changed<Interaction>>,
    _text_query: Query<&mut bevy_ui::widget::Text>,
    _node_query: Query<&mut Node>,
) {
    for (interaction, mut section) in collapsible_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            // Toggle the expansion state
            section.is_expanded = !section.is_expanded;
            
            bevy_log::info!("Toggled section '{}' to {}", 
                section.component_name.split("::").last().unwrap_or("Unknown"),
                if section.is_expanded { "expanded" } else { "collapsed" }
            );
            
            // Update the arrow symbol in the header text
            if let Some(content_entity) = section.content_entity {
                // Find the header text (it's a child of the button that was clicked)
                // We need to look for text entities that are children of this collapsible section
                for _entity_with_text in _text_query.iter() {
                    // This is a simplified approach - in practice you'd track the header text entity
                    // For now, we'll update visibility of the content instead
                }
                
                // Toggle visibility of the content container
                if let Ok(mut content_node) = commands.get_entity(content_entity) {
                    if section.is_expanded {
                        content_node.insert(Node {
                            display: bevy_ui::Display::Flex,
                            ..Default::default()
                        });
                    } else {
                        content_node.insert(Node {
                            display: bevy_ui::Display::None,
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }
}

/// System to monitor entity creation and help debug performance issues.
pub fn debug_entity_count(
    all_entities: Query<Entity>,
    inspector_entities: Query<Entity, With<InspectorEntity>>,
    mut last_total: Local<usize>,
    mut last_inspector: Local<usize>,
) {
    let total_count = all_entities.iter().count();
    let inspector_count = inspector_entities.iter().count();
    
    if total_count != *last_total || inspector_count != *last_inspector {
        bevy_log::info!("Entity counts - Total: {} ({:+}), Inspector: {} ({:+})", 
                       total_count, total_count as i32 - *last_total as i32,
                       inspector_count, inspector_count as i32 - *last_inspector as i32);
        *last_total = total_count;
        *last_inspector = inspector_count;
    }
}

