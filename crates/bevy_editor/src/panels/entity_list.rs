//! Entity list panel for browsing world entities

use bevy::prelude::*;
use bevy::ui::{AlignItems, FlexDirection, UiRect, Val};
use crate::{
    themes::DarkTheme,
    remote::types::{EditorState, RemoteEntity},
    widgets::{
        simple_scrollable::ScrollableContainerPlugin,
        spawn_basic_panel,
        EditorTheme,
    },
};

/// Plugin for entity list functionality
pub struct EntityListPlugin;

impl Plugin for EntityListPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ScrollableContainerPlugin)
            .add_systems(Update, (
                handle_entity_selection,
                update_entity_button_colors,
                refresh_entity_list,
            ));
    }
}

/// Component for marking UI elements - kept for backward compatibility
#[derive(Component)]
pub struct EntityListItem {
    pub entity_id: u32,
}

/// Component for marking UI areas - kept for backward compatibility
#[derive(Component)]
pub struct EntityTree;

#[derive(Component)]
pub struct EntityListArea;

/// Component for marking scrollable areas - kept for backward compatibility
#[derive(Component)]
pub struct ScrollableArea;

/// Component for marking the entity list scrollable area - kept for backward compatibility
#[derive(Component)]
pub struct EntityListScrollArea;

/// Creates the entity list panel using basic widgets
pub fn create_modern_entity_list_panel(
    commands: &mut Commands,
    _theme: &EditorTheme,
) -> Entity {
    spawn_basic_panel(commands, "Entities")
}

/// Handle entity selection in the UI - legacy system
pub fn handle_entity_selection(
    mut interaction_query: Query<
        (&Interaction, &EntityListItem, &mut BackgroundColor), 
        (Changed<Interaction>, With<Button>)
    >,
    mut editor_state: ResMut<EditorState>,
) {
    for (interaction, list_item, mut bg_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                editor_state.selected_entity_id = Some(list_item.entity_id);
                editor_state.show_components = true;
                *bg_color = BackgroundColor(DarkTheme::BUTTON_SELECTED);
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(DarkTheme::BUTTON_HOVER);
            }
            Interaction::None => {
                if Some(list_item.entity_id) == editor_state.selected_entity_id {
                    *bg_color = BackgroundColor(DarkTheme::BUTTON_SELECTED);
                } else {
                    *bg_color = BackgroundColor(DarkTheme::BUTTON_DEFAULT);
                }
            }
        }
    }
}

/// Update entity button colors based on selection state - legacy system
pub fn update_entity_button_colors(
    editor_state: Res<EditorState>,
    mut button_query: Query<(&EntityListItem, &mut BackgroundColor, &Interaction), With<Button>>,
) {
    if !editor_state.is_changed() {
        return;
    }

    for (list_item, mut bg_color, interaction) in &mut button_query {
        if *interaction == Interaction::Hovered {
            continue;
        }
        
        let new_color = if Some(list_item.entity_id) == editor_state.selected_entity_id {
            DarkTheme::BUTTON_SELECTED
        } else {
            DarkTheme::BUTTON_DEFAULT
        };
        
        *bg_color = BackgroundColor(new_color);
    }
}

/// Refresh the entity list display - updated to work with both old and new systems
pub fn refresh_entity_list(
    editor_state: Res<EditorState>,
    mut commands: Commands,
    entity_list_area_query: Query<Entity, With<EntityListArea>>,
    list_items_query: Query<Entity, With<EntityListItem>>,
    mut local_entity_count: Local<usize>,
) {
    let current_count = editor_state.entities.len();
    if *local_entity_count == current_count {
        return;
    }
    *local_entity_count = current_count;

    // Clear existing list items
    for entity in &list_items_query {
        commands.entity(entity).despawn();
    }

    // Find the entity list area and add new items
    for list_area_entity in entity_list_area_query.iter() {
        commands.entity(list_area_entity).despawn_children();
        
        commands.entity(list_area_entity).with_children(|parent| {
            if editor_state.entities.is_empty() {
                // Show empty state with themed styling
                parent.spawn((
                    Text::new("No entities connected.\nStart a bevy_remote server to see entities."),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(DarkTheme::TEXT_MUTED),
                    Node {
                        padding: UiRect::all(Val::Px(16.0)),
                        ..default()
                    },
                ));
            } else {
                // Add entity items using the legacy system for now
                for remote_entity in &editor_state.entities {
                    create_entity_list_item(parent, remote_entity, &editor_state);
                }
            }
        });
    }
}

/// Create a single entity list item - legacy implementation with theme integration
fn create_entity_list_item(parent: &mut ChildSpawnerCommands, remote_entity: &RemoteEntity, editor_state: &EditorState) {
    let bg_color = if Some(remote_entity.id) == editor_state.selected_entity_id {
        Color::srgb(0.3, 0.4, 0.6) // Selection color
    } else {
        Color::srgb(0.15, 0.15, 0.15) // Background tertiary
    };

    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(32.0),
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(10.0)),
                margin: UiRect::bottom(Val::Px(2.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(bg_color),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.3)), // Border color
            EntityListItem { entity_id: remote_entity.id },
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
            )).with_children(|parent| {
                let display_name = format!("Entity {}", remote_entity.id);
                
                parent.spawn((
                    Text::new(display_name),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(DarkTheme::TEXT_PRIMARY),
                ));
            });
        });
}

/// Helper function to create a modern entity list that could be extracted to bevy_feathers
/// This is a placeholder for future implementation
pub fn spawn_modern_entity_list(
    commands: &mut Commands,
    entities: Vec<RemoteEntity>,
    _theme: &EditorTheme,
) -> Entity {
    // For now, just create a simple container
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(DarkTheme::BACKGROUND_PRIMARY),
        ))
        .with_children(|parent| {
            for entity in entities {
                parent.spawn((
                    Text::new(format!("Entity {}", entity.id)),
                    TextFont { font_size: 13.0, ..default() },
                    TextColor(DarkTheme::TEXT_PRIMARY),
                ));
            }
        })
        .id()
}
