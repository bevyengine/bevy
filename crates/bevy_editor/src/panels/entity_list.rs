//! Entity list panel for browsing world entities

use bevy::prelude::*;
use bevy::ui::{FlexDirection, UiRect, Val};
use crate::{
    themes::DarkTheme,
    remote::types::{EditorState, RemoteEntity},
    remote::entity_grouping::group_entities_by_component,
    widgets::{
        spawn_basic_panel,
        EditorTheme,
        ListView,
        ListViewPlugin,
        spawn_list_view,
        EntityListItem,
        EntityTreeGroup,
        TreeViewPlugin,
        spawn_entity_tree_view,
    },
};

/// Plugin for entity list functionality
pub struct EntityListPlugin;

impl Plugin for EntityListPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((ListViewPlugin, TreeViewPlugin))
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

/// Entity list view mode
#[derive(Component, Default, PartialEq, Debug)]
pub enum EntityListViewMode {
    #[default]
    Flat,
    Hierarchical,
}

/// Component for the view mode toggle button
#[derive(Component)]
pub struct ViewModeToggle;

/// Creates the entity list panel using basic widgets
pub fn create_modern_entity_list_panel(
    commands: &mut Commands,
    _theme: &EditorTheme,
) -> Entity {
    // Create the main panel
    let panel_entity = spawn_basic_panel(commands, "Entities");
    
    // Add the view mode toggle button to the panel header
    commands.entity(panel_entity).with_children(|parent| {
        // Header with toggle button
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(30.0),
                flex_direction: FlexDirection::Row,
                align_items: bevy::ui::AlignItems::Center,
                justify_content: bevy::ui::JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(DarkTheme::BACKGROUND_SECONDARY),
        )).with_children(|header| {
            // Toggle button
            header.spawn((
                Button,
                Node {
                    width: Val::Px(60.0),
                    height: Val::Px(24.0),
                    justify_content: bevy::ui::JustifyContent::Center,
                    align_items: bevy::ui::AlignItems::Center,
                    ..default()
                },
                BackgroundColor(DarkTheme::BUTTON_DEFAULT),
                ViewModeToggle,
            )).with_children(|button| {
                button.spawn((
                    Text::new("List"),
                    TextFont { font_size: 10.0, ..default() },
                    TextColor(DarkTheme::TEXT_PRIMARY),
                ));
            });
        });
        
        // Entity list area
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(DarkTheme::BACKGROUND_PRIMARY),
            EntityListArea,
            EntityListViewMode::default(),
        ));
    });
    
    panel_entity
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

/// Handle view mode toggle button clicks
pub fn handle_view_mode_toggle(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<ViewModeToggle>)>,
    mut view_mode_query: Query<&mut EntityListViewMode>,
    button_query: Query<&Children, With<ViewModeToggle>>,
    mut text_query: Query<&mut Text>,
) {
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            for mut view_mode in view_mode_query.iter_mut() {
                // Toggle between modes
                *view_mode = match *view_mode {
                    EntityListViewMode::Flat => EntityListViewMode::Hierarchical,
                    EntityListViewMode::Hierarchical => EntityListViewMode::Flat,
                };
                
                // Update button text - find the text child of the button
                for children in button_query.iter() {
                    for child in children.iter() {
                        if let Ok(mut text) = text_query.get_mut(child) {
                            **text = match *view_mode {
                                EntityListViewMode::Flat => "List".to_string(),
                                EntityListViewMode::Hierarchical => "Tree".to_string(),
                            };
                        }
                    }
                }
            }
        }
    }
}

/// Refresh the entity list display - updated to use generic ListView
pub fn refresh_entity_list(
    editor_state: Res<EditorState>,
    mut commands: Commands,
    entity_list_area_query: Query<(Entity, &EntityListViewMode), (With<EntityListArea>, Changed<EntityListViewMode>)>,
    all_entity_list_areas: Query<(Entity, &EntityListViewMode), With<EntityListArea>>,
    list_items_query: Query<Entity, With<EntityListItem>>,
    mut local_entity_count: Local<usize>,
) {
    let current_count = editor_state.entities.len();
    let count_changed = *local_entity_count != current_count;
    let view_mode_changed = !entity_list_area_query.is_empty();
    
    if !count_changed && !view_mode_changed {
        return;
    }
    
    *local_entity_count = current_count;

    // Use all entity list areas (not just changed ones) for actual processing
    for (list_area_entity, view_mode) in all_entity_list_areas.iter() {
        // Clear existing list items first
        for entity in &list_items_query {
            commands.entity(entity).despawn();
        }
        
        // Clear any existing children from the list area
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
            match view_mode {
                EntityListViewMode::Flat => {
                    create_flat_entity_list(&mut commands, list_area_entity, &editor_state.entities);
                }
                EntityListViewMode::Hierarchical => {
                    create_hierarchical_entity_list(&mut commands, list_area_entity, &editor_state.entities);
                }
            }
        }
    }
}

/// Create a flat entity list using the original ListView approach
fn create_flat_entity_list(
    commands: &mut Commands,
    list_area_entity: Entity,
    entities: &[RemoteEntity],
) {
    // Use the generic ListView system
    let mut entities_sorted = entities.to_vec();
    
    // Sort entities: meaningful names first, then by entity ID
    entities_sorted.sort_by(|a, b| {
        use crate::remote::entity_naming::entity_has_meaningful_name;
        
        let a_has_name = entity_has_meaningful_name(a, None);
        let b_has_name = entity_has_meaningful_name(b, None);
        
        match (a_has_name, b_has_name) {
            (true, false) => std::cmp::Ordering::Less,    // a comes first
            (false, true) => std::cmp::Ordering::Greater, // b comes first
            _ => a.id.cmp(&b.id),                        // same category, sort by ID
        }
    });
    
    let entity_items: Vec<EntityListItem> = entities_sorted
        .iter()
        .map(EntityListItem::from_remote_entity)
        .collect();
        
    let list_view = ListView::new(entity_items.clone())
        .with_item_height(32.0)
        .with_selection_highlight(true);
        
    let list_entity = spawn_list_view(commands, entity_items, list_view);
    commands.entity(list_area_entity).add_child(list_entity);
}

/// Create a hierarchical entity list using the TreeView
fn create_hierarchical_entity_list(
    commands: &mut Commands,
    list_area_entity: Entity,
    entities: &[RemoteEntity],
) {
    // Group entities by component types
    let entity_groups = group_entities_by_component(entities);
    
    // Convert EntityGroups to EntityTreeGroups
    let tree_groups: Vec<EntityTreeGroup> = entity_groups
        .into_iter()
        .map(|group| {
            let items: Vec<EntityListItem> = group.entities
                .iter()
                .map(EntityListItem::from_remote_entity)
                .collect();
                
            EntityTreeGroup {
                name: group.group_name.clone(),
                is_expanded: group.is_expanded,
                items,
                group_id: format!("{:?}", group.group_type), // Use debug format as unique ID
            }
        })
        .collect();
    
    // Create the tree view
    let tree_view_entity = spawn_entity_tree_view(commands, tree_groups);
    commands.entity(list_area_entity).add_child(tree_view_entity);
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
