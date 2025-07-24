//! Inspector Systems

use bevy_ecs::entity::Entity;
use bevy_ecs::query::{Changed, With, Without};
use bevy_ecs::relationship::RelatedSpawnerCommands;
use bevy_ecs::system::{Commands, Query, Res, ResMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::archetype::Archetype;
use bevy_ecs::world::World;
use bevy_input::{keyboard::KeyCode, ButtonInput};
use bevy_time::Time;
use bevy_ui::{Interaction, Node, PositionType, Val, FlexDirection, UiRect};
use bevy_window::Window;
use bevy_log::info;
use bevy_reflect::{Reflect, TypeRegistry};
use std::collections::HashMap;

use super::components::*;
use super::config::InspectorConfig;
use super::data_sources::*;
use super::ui_widgets::*;

/// System to handle inspector window toggle
pub fn handle_inspector_toggle(
    keyboard: Res<ButtonInput<KeyCode>>,
    config: Res<InspectorConfig>,
    mut state: ResMut<InspectorState>,
    mut commands: Commands,
    inspector_windows: Query<Entity, With<InspectorWindowRoot>>,
) {
    if keyboard.just_pressed(config.toggle_key) {
        info!("Toggling inspector window visibility");
        state.window_visible = !state.window_visible;
        
        if !state.window_visible {
            // Close inspector window and any associated entities
            for window_entity in inspector_windows.iter() {
                commands.entity(window_entity).despawn();
            }
            // Also close the inspector window itself if we stored its entity
            if let Some(window_entity) = state.inspector_window_entity {
                commands.entity(window_entity).despawn();
                state.inspector_window_entity = None;
            }
        }
    }
}

/// System to spawn the main inspector window
pub fn spawn_inspector_window(
    mut commands: Commands,
    config: Res<InspectorConfig>,
    state: ResMut<InspectorState>,
    inspector_windows: Query<Entity, With<InspectorWindowRoot>>,
    windows: Query<Entity, With<Window>>,
) {
    if !state.window_visible || !inspector_windows.is_empty() {
        return;
    }

    if let Some(primary_window) = windows.iter().next() {
        info!("Spawning inspector window in separate overlay");

        // Modern sidebar layout: left = entity list, right = details panel
        let _inspector_ui_root = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: bevy_ui::PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    flex_direction: FlexDirection::Row,
                    ..Default::default()
                },
                bevy_ui::BackgroundColor(bevy_color::Color::srgba(0.07, 0.09, 0.13, 0.98)), // Modern dark background
                bevy_ui::ZIndex(1000),
                InspectorWindowRoot {
                    window_entity: primary_window,
                },
                InspectorMarker,
            ))
            .with_children(|parent| {
                // Sidebar: Entity list
                parent.spawn((
                    Node {
                        width: Val::Px(340.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        border: UiRect::right(Val::Px(1.0)),
                        ..Default::default()
                    },
                    bevy_ui::BackgroundColor(bevy_color::Color::srgba(0.13, 0.15, 0.19, 1.0)),
                    InspectorMarker,
                )).with_children(|sidebar| {
                    // Title bar
                    spawn_title_bar(sidebar, &config);
                    // Search box
                    sidebar.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(36.0),
                            padding: UiRect::all(Val::Px(config.styling.padding)),
                            margin: UiRect::all(Val::Px(config.styling.margin)),
                            ..Default::default()
                        },
                        bevy_ui::BackgroundColor(bevy_color::Color::srgba(0.18, 0.20, 0.25, 1.0)),
                        InspectorMarker,
                    )).with_children(|search_parent| {
                        search_parent.spawn((
                            bevy_ui::widget::Text::new("🔍 Search entities..."),
                            bevy_text::TextFont {
                                font_size: config.styling.font_size_normal,
                                ..Default::default()
                            },
                            bevy_text::TextColor(bevy_color::Color::srgba(0.7, 0.7, 0.7, 1.0)),
                            InspectorMarker,
                        ));
                    });
                    // Entity list area
                    sidebar.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            overflow: bevy_ui::Overflow::clip_y(),
                            ..Default::default()
                        },
                        bevy_ui::BackgroundColor(bevy_color::Color::srgba(0.13, 0.15, 0.19, 1.0)),
                        InspectorTreeRoot,
                        InspectorMarker,
                    ));
                });
                // Details panel
                parent.spawn((
                    Node {
                        width: Val::Auto,
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        margin: UiRect::all(Val::Px(12.0)),
                        ..Default::default()
                    },
                    bevy_ui::BackgroundColor(bevy_color::Color::srgba(0.10, 0.11, 0.15, 1.0)),
                    InspectorDetailsPanel,
                    InspectorMarker,
                ));
            })
            .id();
    }
}

/// System to update the inspector tree content
pub fn update_inspector_content(
    mut commands: Commands,
    config: Res<InspectorConfig>,
    mut state: ResMut<InspectorState>,
    _data_source: ResMut<InspectorDataSourceResource>,
    tree_roots: Query<Entity, With<InspectorTreeRoot>>,
    _time: Res<Time>,
    // Query all entities except inspector entities
    all_entities: Query<(Entity, Option<&Name>), Without<InspectorMarker>>,
) {
    // Check if we need to refresh - also force refresh if inspector was just opened
    let should_refresh = (config.auto_refresh_interval > 0.0
        && state.last_refresh.elapsed().as_secs_f32() > config.auto_refresh_interval)
        || state.last_refresh.elapsed().as_secs() == 0; // Force initial refresh
    
    if !should_refresh {
        return;
    }
    
    // Fetch entity data using queries instead of world access
    let mut entities = Vec::new();
    for (entity, name) in all_entities.iter() {
        entities.push(EntityData {
            id: entity,
            name: name.map(|n| n.to_string()),
            components: vec![], // TODO: Implement component detection using queries
            archetype_id: 0, // TODO: Get archetype info if needed
        });
    }
    
    info!("Found {} entities to inspect", entities.len());
    
    // Improved grouping logic
    let mut entity_groups: HashMap<String, Vec<Entity>> = HashMap::new();
    
    // Group entities by name or type
    for entity in &entities {
        match &entity.name {
            Some(name) => {
                // Named entities get their own group or are grouped by name prefix
                let group_name = if name.contains("Player") {
                    "🎮 Players".to_string()
                } else if name.contains("Camera") {
                    "📷 Cameras".to_string()
                } else if name.contains("Light") {
                    "💡 Lights".to_string()
                } else if name.contains("UI") || name.contains("Button") || name.contains("Text") {
                    "🖼️ UI Elements".to_string()
                } else {
                    "📦 Named Entities".to_string()
                };
                entity_groups.entry(group_name).or_default().push(entity.id);
            }
            None => {
                // Unnamed entities grouped together
                entity_groups.entry("❓ Unnamed Entities".to_string()).or_default().push(entity.id);
            }
        }
    }
    
    // Ensure groups are always expanded by default for a better first-time experience
    for group_name in entity_groups.keys() {
        if !state.expanded_groups.contains(group_name) {
            state.expanded_groups.insert(group_name.clone());
        }
    }
    
    state.entity_groups = entity_groups;
    info!("Grouped entities into {} groups: {:?}", state.entity_groups.len(), state.entity_groups.keys().collect::<Vec<_>>());
    
    // Update tree UI
    for tree_root in tree_roots.iter() {
        commands.entity(tree_root).despawn_children();
        
        // Spawn groups
        for (group_name, group_entities) in &state.entity_groups {
            let is_expanded = state.is_group_expanded(group_name);
            
            // Create group header
            commands.entity(tree_root).with_children(|parent| {
                // Group disclosure triangle
                parent.spawn((
                    bevy_ui::widget::Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(32.0),
                        flex_direction: FlexDirection::Row,
                        align_items: bevy_ui::AlignItems::Center,
                        padding: UiRect::all(Val::Px(config.styling.padding)),
                        margin: UiRect::all(Val::Px(config.styling.margin)),
                        ..Default::default()
                    },
                    bevy_ui::BackgroundColor(bevy_color::Color::srgba(
                        config.styling.header_color.0,
                        config.styling.header_color.1,
                        config.styling.header_color.2,
                        config.styling.header_color.3,
                    )),
                    DisclosureTriangleWidget {
                        target_group: group_name.clone(),
                        is_expanded,
                    },
                    InspectorMarker,
                )).with_children(|triangle_parent| {
                    let triangle_symbol = if is_expanded { "▼" } else { "▶" };
                    let text = format!("{} {} ({})", triangle_symbol, group_name, group_entities.len());
                    
                    triangle_parent.spawn((
                        bevy_ui::widget::Text::new(text),
                        bevy_text::TextFont {
                            font_size: config.styling.font_size_normal,
                            ..Default::default()
                        },
                        bevy_text::TextColor(bevy_color::Color::srgba(
                            config.styling.text_color.0,
                            config.styling.text_color.1,
                            config.styling.text_color.2,
                            config.styling.text_color.3,
                        )),
                        InspectorMarker,
                    ));
                });
                
                // Create group content if expanded
                if is_expanded {
                    parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::left(Val::Px(20.0)), // Indent
                            ..Default::default()
                        },
                        InspectorMarker,
                    )).with_children(|content_parent| {
                        // Add entity items
                        for entity_id in group_entities.iter().take(config.max_entities_per_group) {
                            if let Some(entity_data) = entities.iter().find(|e| &e.id == entity_id) {
                                let display_name = entity_data.name.as_deref().unwrap_or("Unnamed Entity");
                                let is_selected = state.selected_entity == Some(*entity_id);
                                
                                content_parent.spawn((
                                    bevy_ui::widget::Button,
                                    Node {
                                        width: Val::Percent(100.0),
                                        min_height: Val::Px(24.0),
                                        flex_direction: FlexDirection::Column,
                                        padding: UiRect::all(Val::Px(config.styling.padding / 2.0)),
                                        margin: UiRect::vertical(Val::Px(1.0)),
                                        ..Default::default()
                                    },
                                    bevy_ui::BackgroundColor(if is_selected {
                                        bevy_color::Color::srgba(
                                            config.styling.highlight_color.0,
                                            config.styling.highlight_color.1,
                                            config.styling.highlight_color.2,
                                            0.3,
                                        )
                                    } else {
                                        bevy_color::Color::NONE
                                    }),
                                    EntityListItem {
                                        entity_id: *entity_id,
                                        display_name: display_name.to_string(),
                                        is_selected,
                                    },
                                    InspectorMarker,
                                )).with_children(|entity_parent| {
                                    entity_parent.spawn((
                                        bevy_ui::widget::Text::new(format!("• {} [{}]", display_name, entity_id.index())),
                                        bevy_text::TextFont {
                                            font_size: config.styling.font_size_normal,
                                            ..Default::default()
                                        },
                                        bevy_text::TextColor(bevy_color::Color::srgba(
                                            config.styling.text_color.0,
                                            config.styling.text_color.1,
                                            config.styling.text_color.2,
                                            config.styling.text_color.3,
                                        )),
                                        InspectorMarker,
                                    ));
                                });
                            }
                        }
                        
                        if group_entities.len() > config.max_entities_per_group {
                            // Add "show more" indicator
                            content_parent.spawn((
                                bevy_ui::widget::Text::new(format!(
                                    "... and {} more entities",
                                    group_entities.len() - config.max_entities_per_group
                                )),
                                bevy_text::TextFont {
                                    font_size: config.styling.font_size_small,
                                    ..Default::default()
                                },
                                bevy_text::TextColor(bevy_color::Color::srgba(0.7, 0.7, 0.7, 1.0)),
                                InspectorMarker,
                            ));
                        }
                    });
                }
            });
        }
    }
    
    state.last_refresh = std::time::Instant::now();
}

/// System to handle disclosure triangle interactions
pub fn handle_disclosure_interactions(
    mut state: ResMut<InspectorState>,
    mut disclosure_query: Query<(&DisclosureTriangleWidget, &Interaction), Changed<Interaction>>,
) {
    for (triangle, interaction) in disclosure_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            state.toggle_group(&triangle.target_group);
        }
    }
}

/// System to handle entity selection
pub fn handle_entity_selection(
    mut state: ResMut<InspectorState>,
    mut entity_items: Query<(&EntityListItem, &Interaction), Changed<Interaction>>,
) {
    for (item, interaction) in entity_items.iter_mut() {
        if *interaction == Interaction::Pressed {
            if state.selected_entity == Some(item.entity_id) {
                state.clear_selection();
            } else {
                state.select_entity(item.entity_id);
            }
        }
    }
}

/// System to handle detailed component inspection
pub fn handle_detailed_view(
    mut commands: Commands,
    config: Res<InspectorConfig>,
    state: Res<InspectorState>,
    _data_source: Res<InspectorDataSourceResource>,
    details_panels: Query<Entity, With<InspectorDetailsPanel>>,
) {
    for details_panel in details_panels.iter() {
        commands.entity(details_panel).despawn_children();
        
        commands.entity(details_panel).with_children(|parent| {
            if let Some(selected_entity) = state.selected_entity {
                // Header with entity info
                parent.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(48.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(config.styling.padding)),
                        margin: UiRect::all(Val::Px(8.0)),
                        ..Default::default()
                    },
                    bevy_ui::BackgroundColor(bevy_color::Color::srgba(0.15, 0.17, 0.22, 1.0)),
                    InspectorMarker,
                )).with_children(|header| {
                    header.spawn((
                        bevy_ui::widget::Text::new(format!("Entity Details")),
                        bevy_text::TextFont {
                            font_size: config.styling.font_size_header,
                            ..Default::default()
                        },
                        bevy_text::TextColor(bevy_color::Color::srgba(0.9, 0.9, 0.9, 1.0)),
                        InspectorMarker,
                    ));
                    header.spawn((
                        bevy_ui::widget::Text::new(format!("ID: {} ({})", selected_entity.index(), selected_entity.generation())),
                        bevy_text::TextFont {
                            font_size: config.styling.font_size_normal,
                            ..Default::default()
                        },
                        bevy_text::TextColor(bevy_color::Color::srgba(0.7, 0.7, 0.7, 1.0)),
                        InspectorMarker,
                    ));
                });

                // Components section
                parent.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(config.styling.padding)),
                        ..Default::default()
                    },
                    InspectorMarker,
                )).with_children(|components_section| {
                    components_section.spawn((
                        bevy_ui::widget::Text::new("Components"),
                        bevy_text::TextFont {
                            font_size: config.styling.font_size_header,
                            ..Default::default()
                        },
                        bevy_text::TextColor(bevy_color::Color::srgba(0.9, 0.9, 0.9, 1.0)),
                        InspectorMarker,
                    ));

                    // Add some sample component data for now
                    let sample_components = vec![
                        ("Transform", true),
                        ("GlobalTransform", true),
                        ("Visibility", true),
                        ("ViewVisibility", false),
                        ("InheritedVisibility", false),
                    ];

                    for (component_name, is_reflected) in sample_components {
                        components_section.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(Val::Px(8.0)),
                                margin: UiRect::vertical(Val::Px(4.0)),
                                ..Default::default()
                            },
                            bevy_ui::BackgroundColor(bevy_color::Color::srgba(0.12, 0.14, 0.18, 1.0)),
                            InspectorMarker,
                        )).with_children(|component_panel| {
                            // Component header
                            component_panel.spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    align_items: bevy_ui::AlignItems::Center,
                                    column_gap: Val::Px(8.0),
                                    ..Default::default()
                                },
                                InspectorMarker,
                            )).with_children(|header| {
                                // Component name
                                header.spawn((
                                    bevy_ui::widget::Text::new(component_name),
                                    bevy_text::TextFont {
                                        font_size: config.styling.font_size_normal,
                                        ..Default::default()
                                    },
                                    bevy_text::TextColor(bevy_color::Color::srgba(0.85, 0.85, 0.85, 1.0)),
                                    InspectorMarker,
                                ));

                                // Reflected badge
                                let badge_color = if is_reflected {
                                    bevy_color::Color::srgba(0.2, 0.7, 0.3, 1.0)
                                } else {
                                    bevy_color::Color::srgba(0.7, 0.5, 0.2, 1.0)
                                };
                                let badge_text = if is_reflected { "Reflected" } else { "Opaque" };

                                header.spawn((
                                    Node {
                                        padding: UiRect::new(Val::Px(6.0), Val::Px(6.0), Val::Px(2.0), Val::Px(2.0)),
                                        ..Default::default()
                                    },
                                    bevy_ui::BackgroundColor(badge_color),
                                    InspectorMarker,
                                )).with_children(|badge| {
                                    badge.spawn((
                                        bevy_ui::widget::Text::new(badge_text),
                                        bevy_text::TextFont {
                                            font_size: config.styling.font_size_small,
                                            ..Default::default()
                                        },
                                        bevy_text::TextColor(bevy_color::Color::WHITE),
                                        InspectorMarker,
                                    ));
                                });
                            });

                            // Component fields (for reflected components)
                            if is_reflected {
                                component_panel.spawn((
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        padding: UiRect::top(Val::Px(8.0)),
                                        row_gap: Val::Px(4.0),
                                        ..Default::default()
                                    },
                                    InspectorMarker,
                                )).with_children(|fields| {
                                    // Sample field data
                                    let sample_fields = match component_name {
                                        "Transform" => vec![
                                            ("translation", "Vec3(0.0, 0.0, 0.0)"),
                                            ("rotation", "Quat(0.0, 0.0, 0.0, 1.0)"),
                                            ("scale", "Vec3(1.0, 1.0, 1.0)"),
                                        ],
                                        "Visibility" => vec![
                                            ("visibility", "Inherited"),
                                        ],
                                        _ => vec![("...", "...")],
                                    };

                                    for (field_name, field_value) in sample_fields {
                                        fields.spawn((
                                            Node {
                                                flex_direction: FlexDirection::Row,
                                                justify_content: bevy_ui::JustifyContent::SpaceBetween,
                                                padding: UiRect::all(Val::Px(4.0)),
                                                ..Default::default()
                                            },
                                            bevy_ui::BackgroundColor(bevy_color::Color::srgba(0.08, 0.10, 0.14, 1.0)),
                                            InspectorMarker,
                                        )).with_children(|field| {
                                            field.spawn((
                                                bevy_ui::widget::Text::new(field_name),
                                                bevy_text::TextFont {
                                                    font_size: config.styling.font_size_small,
                                                    ..Default::default()
                                                },
                                                bevy_text::TextColor(bevy_color::Color::srgba(0.75, 0.75, 0.75, 1.0)),
                                                InspectorMarker,
                                            ));
                                            field.spawn((
                                                bevy_ui::widget::Text::new(field_value),
                                                bevy_text::TextFont {
                                                    font_size: config.styling.font_size_small,
                                                    ..Default::default()
                                                },
                                                bevy_text::TextColor(bevy_color::Color::srgba(0.6, 0.8, 0.9, 1.0)),
                                                InspectorMarker,
                                            ));
                                        });
                                    }
                                });
                            }
                        });
                    }
                });
            } else {
                // No entity selected - show placeholder
                parent.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: bevy_ui::AlignItems::Center,
                        justify_content: bevy_ui::JustifyContent::Center,
                        padding: UiRect::all(Val::Px(config.styling.padding * 2.0)),
                        ..Default::default()
                    },
                    InspectorMarker,
                )).with_children(|placeholder| {
                    placeholder.spawn((
                        bevy_ui::widget::Text::new("🔍"),
                        bevy_text::TextFont {
                            font_size: 48.0,
                            ..Default::default()
                        },
                        bevy_text::TextColor(bevy_color::Color::srgba(0.4, 0.4, 0.4, 1.0)),
                        InspectorMarker,
                    ));
                    placeholder.spawn((
                        bevy_ui::widget::Text::new("Select an entity to view its components"),
                        bevy_text::TextFont {
                            font_size: config.styling.font_size_normal,
                            ..Default::default()
                        },
                        bevy_text::TextColor(bevy_color::Color::srgba(0.6, 0.6, 0.6, 1.0)),
                        InspectorMarker,
                    ));
                });
            }
        });
    }
}

/// Helper function to spawn title bar
fn spawn_title_bar(parent: &mut RelatedSpawnerCommands<'_, ChildOf>, config: &InspectorConfig) {
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(32.0),
            align_items: bevy_ui::AlignItems::Center,
            justify_content: bevy_ui::JustifyContent::SpaceBetween,
            padding: UiRect::all(Val::Px(config.styling.padding)),
            ..Default::default()
        },
        bevy_ui::BackgroundColor(bevy_color::Color::srgba(
            config.styling.header_color.0,
            config.styling.header_color.1,
            config.styling.header_color.2,
            config.styling.header_color.3,
        )),
        InspectorMarker,
    )).with_children(|title_parent| {
        // Title text
        title_parent.spawn((
            bevy_ui::widget::Text::new(config.window_title.clone()),
            bevy_text::TextFont {
                font_size: config.styling.font_size_header,
                ..Default::default()
            },
            bevy_text::TextColor(bevy_color::Color::srgba(
                config.styling.text_color.0,
                config.styling.text_color.1,
                config.styling.text_color.2,
                config.styling.text_color.3,
            )),
            InspectorMarker,
        ));
        
        // Close button
        title_parent.spawn((
            bevy_ui::widget::Button,
            Node {
                width: Val::Px(24.0),
                height: Val::Px(24.0),
                align_items: bevy_ui::AlignItems::Center,
                justify_content: bevy_ui::JustifyContent::Center,
                ..Default::default()
            },
            bevy_ui::BackgroundColor(bevy_color::Color::srgba(0.8, 0.2, 0.2, 1.0)),
            InspectorMarker,
        )).with_children(|close_parent| {
            close_parent.spawn((
                bevy_ui::widget::Text::new("×"),
                bevy_text::TextFont {
                    font_size: 16.0,
                    ..Default::default()
                },
                bevy_text::TextColor(bevy_color::Color::WHITE),
                InspectorMarker,
            ));
        });
    });
}
