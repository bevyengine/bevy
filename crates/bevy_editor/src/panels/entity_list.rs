//! Entity list panel for browsing world entities

use bevy::prelude::*;
use bevy::ui::{FlexDirection, UiRect, Val};
use crate::{
    themes::DarkTheme,
    remote::types::{EditorState, RemoteEntity},
    widgets::{
        simple_scrollable::ScrollableContainerPlugin,
        spawn_basic_panel,
        EditorTheme,
        ListView,
        ListViewPlugin,
        spawn_list_view,
        EntityListItem,
    },
};

/// Plugin for entity list functionality
pub struct EntityListPlugin;

impl Plugin for EntityListPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((ScrollableContainerPlugin, ListViewPlugin))
            .add_systems(Update, (
                handle_entity_selection,
                update_entity_button_colors,
                refresh_entity_list,
            ));
    }
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

/// Handle entity selection in the UI - updated to work with ListView
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

/// Refresh the entity list display - updated to use generic ListView
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
        
        if editor_state.entities.is_empty() {
            // Show empty state with themed styling
            commands.entity(list_area_entity).with_children(|parent| {
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
            });
        } else {
            // Use the generic ListView system
            let entity_items: Vec<EntityListItem> = editor_state.entities
                .iter()
                .map(EntityListItem::from_remote_entity)
                .collect();
                
            let list_view = ListView::new(entity_items.clone())
                .with_item_height(32.0)
                .with_selection_highlight(true);
                
            let list_entity = spawn_list_view(&mut commands, entity_items, list_view);
            commands.entity(list_area_entity).add_child(list_entity);
        }
    }
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
