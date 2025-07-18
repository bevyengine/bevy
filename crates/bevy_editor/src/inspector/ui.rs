use crate::inspector::{
    events::*,
    selection::SelectedEntity,
    tree::{TreeNodeInteraction, TreeState},
};
use bevy::prelude::*;

/// Marker component for the inspector root
#[derive(Component)]
pub struct InspectorRoot;

/// Marker component for the entity tree
#[derive(Component)]
pub struct EntityTree;

/// Marker component for the component details panel
#[derive(Component)]
pub struct ComponentDetails;

/// Set up the main inspector UI
pub fn setup_inspector(mut commands: Commands) {
    // Create the main inspector layout
    commands
        .spawn((
            InspectorRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
        ))
        .with_children(|parent| {
            // Left panel: Entity tree
            parent
                .spawn((
                    EntityTree,
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        border: UiRect::right(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_children(|parent| {
                    // Tree header
                    parent.spawn((
                        Text::new("Entities"),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        Node {
                            padding: UiRect::all(Val::Px(10.0)),
                            border: UiRect::bottom(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                        BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
                    ));
                });

            // Right panel: Component details
            parent
                .spawn((
                    ComponentDetails,
                    Node {
                        flex_grow: 1.0,
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.12, 0.12)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Select an entity to view components"),
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                    ));
                });
        });
}

/// System that updates the entity tree when new entities are added
pub fn update_entity_tree(
    mut commands: Commands,
    mut inspector_events: EventReader<InspectorEvent>,
    tree_query: Query<Entity, With<EntityTree>>,
    selected_entity: Res<SelectedEntity>,
    tree_state: Res<TreeState>,
) {
    for event in inspector_events.read() {
        match event {
            InspectorEvent::EntitiesAdded(entities) => {
                if let Ok(tree_entity) = tree_query.single() {
                    // Clear existing children (except header)
                    if let Ok(mut entity_commands) = commands.get_entity(tree_entity) {
                        entity_commands.with_children(|parent| {
                            // Keep the header, add entity list
                            for entity_data in entities {
                                spawn_entity_row(parent, &entity_data, &selected_entity, &tree_state);
                            }
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

/// System that handles tree node interactions
pub fn handle_tree_interactions(
    mut commands: Commands,
    mut interaction_events: EventReader<TreeNodeInteraction>,
    mut selected_entity: ResMut<SelectedEntity>,
    details_query: Query<Entity, With<ComponentDetails>>,
) {
    for event in interaction_events.read() {
        // Update selected entity
        selected_entity.0 = Some(Entity::from_bits(event.node_id.parse::<u64>().unwrap_or(0)));
        
        // Update component details panel
        if let Ok(details_entity) = details_query.single() {
            if let Ok(mut entity_commands) = commands.get_entity(details_entity) {
                entity_commands.despawn_descendants();
                entity_commands.with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("Entity: {}", event.node_id)),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        Node {
                            margin: UiRect::bottom(Val::Px(10.0)),
                            ..default()
                        },
                    ));
                    
                    parent.spawn((
                        Text::new("Loading components..."),
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                    ));
                });
            }
        }
    }
}

/// System that updates component details when components change
pub fn update_component_details(
    mut commands: Commands,
    mut inspector_events: EventReader<InspectorEvent>,
    details_query: Query<Entity, With<ComponentDetails>>,
    selected_entity: Res<SelectedEntity>,
) {
    for event in inspector_events.read() {
        if let InspectorEvent::ComponentsChanged { entity, new_components } = event {
            // Only update if this is the selected entity
            if let Some(selected) = selected_entity.0 {
                if selected == *entity {
                    if let Ok(details_entity) = details_query.single() {
                        if let Ok(mut entity_commands) = commands.get_entity(details_entity) {
                            entity_commands.despawn_descendants();
                            entity_commands.with_children(|parent| {
                                parent.spawn((
                                    Text::new(format!("Entity: {:?}", entity)),
                                    TextColor(Color::WHITE),
                                    TextFont {
                                        font_size: 16.0,
                                        ..default()
                                    },
                                    Node {
                                        margin: UiRect::bottom(Val::Px(15.0)),
                                        ..default()
                                    },
                                ));
                                
                                for component in new_components {
                                    spawn_component_details(parent, component);
                                }
                            });
                        }
                    }
                }
            }
        }
    }
}

fn spawn_entity_row(
    parent: &mut ChildBuilder,
    entity_data: &EntityData,
    selected_entity: &SelectedEntity,
    _tree_state: &TreeState,
) {
    let is_selected = selected_entity.0 == Some(entity_data.entity);
    
    // Convert entity to index for display
    let entity_index = entity_data.entity.index();
    
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::vertical(Val::Px(1.0)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(if is_selected {
                Color::srgb(0.3, 0.4, 0.6)
            } else {
                Color::srgb(0.18, 0.18, 0.18)
            }),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("Entity {}", entity_index)),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
            ));
        })
        .observe(move |_trigger: On<Pointer<Click>>, mut events: EventWriter<TreeNodeInteraction>| {
            events.trigger(TreeNodeInteraction {
                node_id: entity_index.to_string(),
            });
        });
}

/// Spawn component details UI
fn spawn_component_details(parent: &mut ChildBuilder, component: &ComponentData) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                margin: UiRect::bottom(Val::Px(10.0)),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.18, 0.18, 0.18)),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
        ))
        .with_children(|parent| {
            // Component type name
            parent.spawn((
                Text::new(&component.type_name),
                TextColor(Color::srgb(0.4, 0.8, 0.4)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                Node {
                    margin: UiRect::bottom(Val::Px(5.0)),
                    ..default()
                },
            ));
            
            // Component data (simplified JSON display)
            let data_text = serde_json::to_string_pretty(&component.data)
                .unwrap_or_else(|_| "Invalid JSON".to_string());
            
            parent.spawn((
                Text::new(data_text),
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
            ));
        });
}
