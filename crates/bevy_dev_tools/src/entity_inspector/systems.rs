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
use bevy_ui::{BackgroundColor, BorderColor, FlexDirection, Interaction, Node, UiTargetCamera, Val};
use bevy_text::Justify;
use bevy_window::{Window, WindowRef};

use super::{ui, InspectorConfig, InspectorData, InspectorState};

/// Macro to conditionally include UiTargetCamera component
macro_rules! with_camera_if_needed {
    ($components:expr, $is_overlay:expr, $camera:expr) => {
        if $is_overlay {
            $components
        } else {
            ($components, UiTargetCamera($camera))
        }
    };
}

/// Helper function to format component data for display with proper truncation and formatting.
fn format_component_data(value_str: &str, max_line_length: usize, max_lines: usize) -> String {
    // Handle very long single-line values (common with Bevy's debug output)
    if value_str.len() > max_line_length * 2 && !value_str.contains('\n') {
        // For very long single lines, just truncate with context
        return format!("{}...\n(truncated, {} total chars)", 
                      &value_str[..max_line_length.min(value_str.len())], 
                      value_str.len());
    }
    
    // Try to make the debug output more readable by adding strategic line breaks
    let mut formatted = value_str.to_string();
    
    // Handle struct-like patterns
    if formatted.contains(" { ") {
        formatted = formatted
            .replace(" { ", " {\n  ")
            .replace(", ", ",\n  ")
            .replace(" }", "\n}");
    }
    
    // Handle array/vec patterns  
    if formatted.contains(": [") {
        formatted = formatted
            .replace(": [", ": [\n    ")
            .replace(", ", ",\n    ")
            .replace(" ]", "\n  ]");
    }
    
    let lines: Vec<&str> = formatted.lines().collect();
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

/// System that displays components of the selected entity using reflection.
pub fn display_entity_components(
    mut commands: Commands,
    inspector_data: Res<InspectorData>,
    current_state: Res<State<InspectorState>>,
    component_viewer: Query<Entity, With<ComponentViewerContainer>>,
    type_registry: Res<AppTypeRegistry>,
    world: &World,
    mut last_selected_entity: Local<Option<Entity>>,
    mut update_cooldown: Local<f32>,
    time: Res<bevy_time::Time>,
) {
    // Only run when inspector is active
    if *current_state.get() != InspectorState::Active {
        return;
    }

    // Only run if we have a component viewer container
    let Ok(viewer_entity) = component_viewer.single() else {
        return;
    };

    // Add cooldown to prevent excessive rebuilds
    *update_cooldown -= time.delta_secs();
    
    // Only rebuild if entity changed AND cooldown expired
    let entity_changed = inspector_data.selected_entity != *last_selected_entity;
    let cooldown_ready = *update_cooldown <= 0.0;
    
    if !entity_changed {
        return;
    }
    
    if !cooldown_ready {
        return;
    }
    
    // Reset cooldown (minimum 250ms between rebuilds)
    *update_cooldown = 0.25;
    *last_selected_entity = inspector_data.selected_entity;

    bevy_log::info!("Building component viewer structure for entity: {:?}", inspector_data.selected_entity);
    
    // Clear existing component display
    if let Ok(mut entity_commands) = commands.get_entity(viewer_entity) {
        entity_commands.clear_children();
    }

    // Display components of selected entity
    if let Some(selected_entity) = inspector_data.selected_entity {
        if let Ok(entity_ref) = world.get_entity(selected_entity) {
            let type_registry = type_registry.read();
            // Get camera entity, use placeholder for overlay mode (components won't be added anyway)
            let camera_entity = inspector_data.inspector_camera.unwrap_or(Entity::PLACEHOLDER);
            let is_overlay_mode = inspector_data.inspector_camera.is_none();
            
            commands.entity(viewer_entity).with_children(|parent| {
                // Display entity ID header
                let mut entity_header = parent.spawn((
                    bevy_ui::widget::Text::new(format!("Entity: {} ({})", selected_entity.index(), selected_entity.generation())),
                    bevy_text::TextFont {
                        font_size: 14.0,
                        ..Default::default()
                    },
                    bevy_text::TextColor(bevy_color::Color::srgb(0.8, 0.8, 1.0)),
                    InspectorEntity,
                ));
                // Only add UiTargetCamera in separate window mode
                if !is_overlay_mode {
                    entity_header.insert(UiTargetCamera(camera_entity));
                }

                // Get archetype to iterate over components
                let archetype = entity_ref.archetype();
                
                for component_id in archetype.components() {
                    if let Some(component_info) = world.components().get_info(component_id) {
                        let component_name = component_info.name();
                        let component_name_str = format!("{}", component_name);
                        
                        // Create a collapsible section for each component
                        parent
                            .spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    margin: bevy_ui::UiRect::bottom(Val::Px(8.0)),
                                    padding: bevy_ui::UiRect::all(Val::Px(4.0)),
                                    flex_direction: FlexDirection::Column,
                                    ..Default::default()
                                },
                                BackgroundColor(bevy_color::Color::srgb(0.15, 0.15, 0.2)),
                                BorderColor::all(bevy_color::Color::srgb(0.3, 0.3, 0.4)),
                                
                                InspectorEntity,
                            ))
                            .with_children(|component_parent| {
                                // Component name header
                                component_parent.spawn((
                                    bevy_ui::widget::Text::new(component_name_str.clone()),
                                    bevy_text::TextFont {
                                        font_size: 12.0,
                                        ..Default::default()
                                    },
                                    bevy_text::TextColor(bevy_color::Color::srgb(0.9, 0.9, 0.6)),
                                    
                                    InspectorEntity,
                                ));

                                // Try to get the component data using reflection
                                if let Some(type_registration) = type_registry.get_with_type_path(&component_name_str) {
                                    if let Some(reflect_component) = type_registration.data::<ReflectComponent>() {
                                        if let Ok(entity_ref) = world.get_entity(selected_entity) {
                                            if let Some(reflected) = reflect_component.reflect(entity_ref) {
                                                // Display the actual component data
                                                let raw_value_str = format!("{:?}", reflected);
                                                let formatted_value = format_component_data(&raw_value_str, 80, 20);
                                                
                                                component_parent
                                                    .spawn((
                                                        Node {
                                                            width: Val::Percent(100.0),
                                                            padding: bevy_ui::UiRect::all(Val::Px(4.0)),
                                                            ..Default::default()
                                                        },
                                                        BackgroundColor(bevy_color::Color::srgb(0.1, 0.1, 0.15)),
                                                        
                                                        InspectorEntity,
                                                    ))
                                                    .with_child((
                                                        bevy_ui::widget::Text::new(formatted_value),
                                                        bevy_text::TextFont {
                                                            font_size: 9.0,
                                                            ..Default::default()
                                                        },
                                                        bevy_text::TextColor(bevy_color::Color::srgb(0.9, 0.9, 0.9)),
                                                        bevy_text::TextLayout::new_with_justify(Justify::Left),
                                                        Node {
                                                            max_width: Val::Percent(100.0),
                                                            ..Default::default()
                                                        },
                                                        ComponentValueText {
                                                            entity: selected_entity,
                                                            component_name: component_name_str.clone(),
                                                        },
                                                        
                                                        InspectorEntity,
                                                    ));
                                            } else {
                                                component_parent.spawn((
                                                    bevy_ui::widget::Text::new("  <not reflectable>"),
                                                    bevy_text::TextFont {
                                                        font_size: 10.0,
                                                        ..Default::default()
                                                    },
                                                    bevy_text::TextColor(bevy_color::Color::srgb(0.6, 0.6, 0.6)),
                                                    
                                                    InspectorEntity,
                                                ));
                                            }
                                        } else {
                                            component_parent.spawn((
                                                bevy_ui::widget::Text::new("  <entity not found>"),
                                                bevy_text::TextFont {
                                                    font_size: 10.0,
                                                    ..Default::default()
                                                },
                                                bevy_text::TextColor(bevy_color::Color::srgb(0.6, 0.6, 0.6)),
                                                
                                                InspectorEntity,
                                            ));
                                        }
                                    } else {
                                        component_parent.spawn((
                                            bevy_ui::widget::Text::new("  <no reflection data>"),
                                            bevy_text::TextFont {
                                                font_size: 10.0,
                                                ..Default::default()
                                            },
                                            bevy_text::TextColor(bevy_color::Color::srgb(0.6, 0.6, 0.6)),
                                            
                                            InspectorEntity,
                                        ));
                                    }
                                } else {
                                    component_parent.spawn((
                                        bevy_ui::widget::Text::new("  <not registered>"),
                                        bevy_text::TextFont {
                                            font_size: 10.0,
                                            ..Default::default()
                                        },
                                        bevy_text::TextColor(bevy_color::Color::srgb(0.6, 0.6, 0.6)),
                                        
                                        InspectorEntity,
                                    ));
                                }
                            });
                    }
                }
            });
        } else {
            bevy_log::warn!("Selected entity {:?} no longer exists", selected_entity);
        }
    } else {
        // No entity selected - show empty state
        commands.entity(viewer_entity).with_children(|parent| {
            parent.spawn((
                bevy_ui::widget::Text::new("No entity selected\n\nClick on an entity in the left pane to inspect its components."),
                bevy_text::TextFont {
                    font_size: 14.0,
                    ..Default::default()
                },
                bevy_text::TextColor(bevy_color::Color::srgb(0.6, 0.6, 0.6)),
                UiTargetCamera(inspector_data.inspector_camera.unwrap_or(Entity::PLACEHOLDER)),
                InspectorEntity,
            ));
        });
    }
}

/// System that updates component values in real-time for the selected entity.
pub fn update_component_values_live(
    mut component_texts: Query<(&mut bevy_ui::widget::Text, &ComponentValueText)>,
    inspector_data: Res<InspectorData>,
    current_state: Res<State<InspectorState>>,
    // Query for common components we want to track
    transforms: Query<&bevy_transform::components::Transform>,
    names: Query<&Name>,
    mut update_timer: Local<f32>,
    time: Res<bevy_time::Time>,
) {
    // Only run when inspector is active
    if *current_state.get() != InspectorState::Active {
        return;
    }

    // Only run if we have a selected entity
    if inspector_data.selected_entity.is_none() {
        return;
    }

    // Update component values every 200ms for smoother updates
    *update_timer -= time.delta_secs();
    if *update_timer > 0.0 {
        return;
    }
    *update_timer = 0.2; // 5 FPS update rate for smoother updates
    
    for (mut text, component_value_text) in component_texts.iter_mut() {
        // Only update components for the currently selected entity
        if Some(component_value_text.entity) != inspector_data.selected_entity {
            continue;
        }
        
        // Update specific components we can query directly
        match component_value_text.component_name.as_str() {
            "bevy_transform::components::transform::Transform" => {
                if let Ok(transform) = transforms.get(component_value_text.entity) {
                    let raw_value_str = format!("{:?}", transform);
                    let formatted_value = format_component_data(&raw_value_str, 80, 20);
                    text.0 = formatted_value;
                }
            }
            "bevy_ecs::name::Name" => {
                if let Ok(name) = names.get(component_value_text.entity) {
                    let raw_value_str = format!("{:?}", name);
                    let formatted_value = format_component_data(&raw_value_str, 80, 20);
                    text.0 = formatted_value;
                }
            }
            _ => {
                // For other components, we can't update them without World access
                // This is a limitation, but at least Transform (the most important one) will update
                if text.0 == "Loading..." {
                    text.0 = "  <live updates limited>".to_string();
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

