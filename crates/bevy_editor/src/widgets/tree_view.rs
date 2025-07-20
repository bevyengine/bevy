//! Tree view widget for hierarchical display of data
//! 
//! This widget provides expandable/collapsible tree-like display suitable
//! for hierarchical data like grouped entities.

use bevy::prelude::*;
use bevy::ui::{UiRect, Val, FlexDirection, AlignItems, JustifyContent};
use crate::themes::DarkTheme;
use crate::widgets::EntityListItem;

/// A tree view widget for displaying hierarchical data
#[derive(Component, Clone)]
pub struct EntityTreeView {
    pub groups: Vec<EntityTreeGroup>,
    pub item_height: f32,
    pub group_header_height: f32,
    pub indent_size: f32,
}

/// A group in the tree view
#[derive(Clone)]
pub struct EntityTreeGroup {
    pub name: String,
    pub is_expanded: bool,
    pub items: Vec<EntityListItem>,
    pub group_id: String,
}

impl EntityTreeView {
    pub fn new(groups: Vec<EntityTreeGroup>) -> Self {
        Self {
            groups,
            item_height: 28.0,
            group_header_height: 32.0,
            indent_size: 16.0,
        }
    }
    
    pub fn with_item_height(mut self, height: f32) -> Self {
        self.item_height = height;
        self
    }
    
    pub fn with_group_header_height(mut self, height: f32) -> Self {
        self.group_header_height = height;
        self
    }
    
    pub fn toggle_group(&mut self, group_id: &str) {
        if let Some(group) = self.groups.iter_mut().find(|g| g.group_id == group_id) {
            group.is_expanded = !group.is_expanded;
        }
    }
    
    pub fn expand_all(&mut self) {
        for group in &mut self.groups {
            group.is_expanded = true;
        }
    }
    
    pub fn collapse_all(&mut self) {
        for group in &mut self.groups {
            group.is_expanded = false;
        }
    }
}

/// Marker component for tree view items
#[derive(Component)]
pub struct TreeViewItem {
    pub group_id: String,
    pub item_index: usize,
    pub is_group_header: bool,
}

/// Marker component for group headers
#[derive(Component)]
pub struct TreeGroupHeader {
    pub group_id: String,
}

/// Plugin for tree view functionality
pub struct TreeViewPlugin;

impl Plugin for TreeViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            handle_tree_group_clicks,
            update_tree_view_display,
        ));
    }
}

/// Handle clicks on tree group headers to expand/collapse groups
fn handle_tree_group_clicks(
    mut tree_query: Query<&mut EntityTreeView>,
    interaction_query: Query<
        (&Interaction, &TreeGroupHeader),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, header) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            for mut tree_view in tree_query.iter_mut() {
                tree_view.toggle_group(&header.group_id);
            }
        }
    }
}

/// Update the visual display when tree view changes
fn update_tree_view_display(
    mut commands: Commands,
    tree_query: Query<(Entity, &EntityTreeView), Or<(Changed<EntityTreeView>, Added<EntityTreeView>)>>,
    item_query: Query<Entity, With<TreeViewItem>>,
) {
    for (tree_entity, tree_view) in tree_query.iter() {
        // Clear existing items
        for item_entity in item_query.iter() {
            commands.entity(item_entity).despawn();
        }
        
        // Rebuild the tree view
        commands.entity(tree_entity).despawn_children();
        commands.entity(tree_entity).with_children(|parent| {
            for group in &tree_view.groups {
                // Inline tree group creation to avoid ChildBuilder type issues
                let expansion_icon = if group.is_expanded { "-" } else { "+" };
                
                // Group header
                parent.spawn((
                    Button,
                    Node {
                        height: Val::Px(tree_view.group_header_height),
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        padding: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(4.0), Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(DarkTheme::BACKGROUND_SECONDARY),
                    TreeGroupHeader { group_id: group.group_id.clone() },
                    TreeViewItem {
                        group_id: group.group_id.clone(),
                        item_index: 0,
                        is_group_header: true,
                    },
                )).with_children(|header| {
                    // Expansion icon
                    header.spawn((
                        Text::new(expansion_icon),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(DarkTheme::TEXT_SECONDARY),
                        Node {
                            width: Val::Px(16.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                    ));
                    
                    // Group name and count
                    header.spawn((
                        Text::new(format!("{} ({})", group.name, group.items.len())),
                        TextFont { 
                            font_size: 13.0,
                            ..default() 
                        },
                        TextColor(DarkTheme::TEXT_PRIMARY),
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                    ));
                });
                
                // Group items (only if expanded)
                if group.is_expanded {
                    for (index, entity_item) in group.items.iter().enumerate() {
                        parent.spawn((
                            Button,
                            Node {
                                height: Val::Px(tree_view.item_height),
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                padding: UiRect::new(
                                    Val::Px(tree_view.indent_size + 8.0), // Indent for hierarchy
                                    Val::Px(8.0), 
                                    Val::Px(2.0), 
                                    Val::Px(2.0)
                                ),
                                ..default()
                            },
                            BackgroundColor(DarkTheme::BUTTON_DEFAULT),
                            TreeViewItem {
                                group_id: group.group_id.clone(),
                                item_index: index,
                                is_group_header: false,
                            },
                            entity_item.clone(),
                        )).with_children(|item_parent| {
                            item_parent.spawn((
                                Text::new(&entity_item.name),
                                TextFont { font_size: 12.0, ..default() },
                                TextColor(DarkTheme::TEXT_PRIMARY),
                            ));
                        });
                    }
                }
            }
        });
    }
}

/// Spawn a tree view from groups
pub fn spawn_entity_tree_view(
    commands: &mut Commands,
    groups: Vec<EntityTreeGroup>,
) -> Entity {
    let entity = commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(DarkTheme::BACKGROUND_PRIMARY),
    )).id();
    
    // Create the tree view component
    let tree_view = EntityTreeView::new(groups.clone());
    commands.entity(entity).insert(tree_view);
    
    // Manually trigger the display creation
    commands.entity(entity).with_children(|parent| {
        for group in &groups {
            let expansion_icon = if group.is_expanded { "-" } else { "+" };
            
            // Group header
            parent.spawn((
                Button,
                Node {
                    height: Val::Px(32.0), // group_header_height
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(4.0), Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(DarkTheme::BACKGROUND_SECONDARY),
                TreeGroupHeader { group_id: group.group_id.clone() },
                TreeViewItem {
                    group_id: group.group_id.clone(),
                    item_index: 0,
                    is_group_header: true,
                },
            )).with_children(|header| {
                // Expansion icon
                header.spawn((
                    Text::new(expansion_icon),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(DarkTheme::TEXT_SECONDARY),
                    Node {
                        width: Val::Px(16.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ));
                
                // Group name and count
                header.spawn((
                    Text::new(format!("{} ({})", group.name, group.items.len())),
                    TextFont { 
                        font_size: 13.0,
                        ..default() 
                    },
                    TextColor(DarkTheme::TEXT_PRIMARY),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                ));
            });
            
            // Group items (only if expanded)
            if group.is_expanded {
                for (index, entity_item) in group.items.iter().enumerate() {
                    parent.spawn((
                        Button,
                        Node {
                            height: Val::Px(28.0), // item_height
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            padding: UiRect::new(
                                Val::Px(16.0 + 8.0), // indent_size + 8.0
                                Val::Px(8.0), 
                                Val::Px(2.0), 
                                Val::Px(2.0)
                            ),
                            ..default()
                        },
                        BackgroundColor(DarkTheme::BUTTON_DEFAULT),
                        TreeViewItem {
                            group_id: group.group_id.clone(),
                            item_index: index,
                            is_group_header: false,
                        },
                        entity_item.clone(),
                    )).with_children(|item_parent| {
                        item_parent.spawn((
                            Text::new(&entity_item.name),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(DarkTheme::TEXT_PRIMARY),
                        ));
                    });
                }
            }
        }
    });
    
    entity
}
