//! Reusable UI Widgets for Inspector

use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::relationship::RelatedSpawnerCommands;
use bevy_ecs::system::Commands;
use bevy_ecs::hierarchy::ChildOf;
use bevy_text::{TextColor, TextFont};
use bevy_ui::prelude::*;
use bevy_ui::widget::{Button, Text};

use super::components::*;
use super::config::InspectorStyling;

/// Reusable disclosure triangle widget
#[derive(Component)]
pub struct DisclosureTriangleWidget {
    pub target_group: String,
    pub is_expanded: bool,
}

/// Reusable collapsible panel widget
#[derive(Component)]
pub struct CollapsiblePanel {
    pub title: String,
    pub is_expanded: bool,
    pub content_entity: Option<Entity>,
}

/// Reusable entity list item widget
#[derive(Component)]
pub struct EntityListItem {
    pub entity_id: Entity,
    pub display_name: String,
    pub is_selected: bool,
}

/// Reusable component badge widget
#[derive(Component)]
pub struct ComponentBadge {
    pub component_name: String,
    pub is_reflected: bool,
}

/// Reusable search box widget
#[derive(Component)]
pub struct SearchBox {
    pub placeholder: String,
    pub current_text: String,
}

/// Reusable property inspector widget for reflected components
#[derive(Component)]
pub struct PropertyInspector {
    pub entity_id: Entity,
    pub component_type_name: String,
}

/// Resizable panel divider/splitter
#[derive(Component)]
pub struct ResizeHandle {
    pub target_panel: Entity,
    pub resize_direction: ResizeDirection,
    pub is_dragging: bool,
    pub last_cursor_pos: Option<bevy_math::Vec2>,
}

/// Direction for resizing panels
#[derive(Debug, Clone)]
pub enum ResizeDirection {
    Horizontal,
    Vertical,
}

/// Resizable panel that can be resized by dragging handles
#[derive(Component)]
pub struct ResizablePanel {
    pub min_size: f32,
    pub max_size: f32,
    pub current_size: f32,
}

impl DisclosureTriangleWidget {
    /// Spawn a disclosure triangle button
    pub fn spawn(
        commands: &mut Commands,
        parent: Entity,
        group_name: &str,
        entity_count: usize,
        is_expanded: bool,
        styling: &InspectorStyling,
    ) -> Entity {
        let triangle_symbol = if is_expanded { "v" } else { ">" };
        let text = format!("{} {} ({})", triangle_symbol, group_name, entity_count);
        
        let entity = commands
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(32.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(styling.padding)),
                    margin: UiRect::all(Val::Px(styling.margin)),
                    ..Default::default()
                },
                BackgroundColor(Color::srgba(
                    styling.header_color.0,
                    styling.header_color.1,
                    styling.header_color.2,
                    styling.header_color.3,
                )),
                DisclosureTriangleWidget {
                    target_group: group_name.to_string(),
                    is_expanded,
                },
                InspectorMarker,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(text),
                    TextFont {
                        font_size: styling.font_size_normal,
                        ..Default::default()
                    },
                    TextColor(Color::srgba(
                        styling.text_color.0,
                        styling.text_color.1,
                        styling.text_color.2,
                        styling.text_color.3,
                    )),
                    InspectorMarker,
                ));
            })
            .id();
            
        commands.entity(parent).add_child(entity);
        entity
    }
}

impl CollapsiblePanel {
    /// Spawn a collapsible panel with header and content area
    pub fn spawn(
        commands: &mut Commands,
        parent: Entity,
        title: &str,
        is_expanded: bool,
        styling: &InspectorStyling,
    ) -> (Entity, Entity) {
        let panel_entity = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    margin: UiRect::all(Val::Px(styling.margin)),
                    ..Default::default()
                },
                BackgroundColor(Color::srgba(
                    styling.background_color.0,
                    styling.background_color.1,
                    styling.background_color.2,
                    styling.background_color.3,
                )),
                CollapsiblePanel {
                    title: title.to_string(),
                    is_expanded,
                    content_entity: None,
                },
                InspectorMarker,
            ))
            .id();
            
        // Header
        let header_entity = commands
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(28.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(styling.padding)),
                    ..Default::default()
                },
                BackgroundColor(Color::srgba(
                    styling.header_color.0,
                    styling.header_color.1,
                    styling.header_color.2,
                    styling.header_color.3,
                )),
                InspectorMarker,
            ))
            .with_children(|parent| {
                let symbol = if is_expanded { "v" } else { ">" };
                parent.spawn((
                    Text::new(format!("{} {}", symbol, title)),
                    TextFont {
                        font_size: styling.font_size_normal,
                        ..Default::default()
                    },
                    TextColor(Color::srgba(
                        styling.text_color.0,
                        styling.text_color.1,
                        styling.text_color.2,
                        styling.text_color.3,
                    )),
                    InspectorMarker,
                ));
            })
            .id();
            
        // Content area (only visible if expanded)
        let content_entity = if is_expanded {
            let content = commands
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(styling.padding)),
                        ..Default::default()
                    },
                    InspectorMarker,
                ))
                .id();
            Some(content)
        } else {
            None
        };
        
        commands.entity(panel_entity).add_child(header_entity);
        if let Some(content) = content_entity {
            commands.entity(panel_entity).add_child(content);
        }
        
        commands.entity(parent).add_child(panel_entity);
        
        // Update the panel component with content entity reference
        if let Some(content) = content_entity {
            commands.entity(panel_entity).insert(CollapsiblePanel {
                title: title.to_string(),
                is_expanded,
                content_entity: Some(content),
            });
        }
        
        (panel_entity, content_entity.unwrap_or(panel_entity))
    }
}

impl EntityListItem {
    /// Spawn an entity list item with selection support
    pub fn spawn(
        commands: &mut Commands,
        parent: Entity,
        entity_id: Entity,
        display_name: &str,
        component_names: &[String],
        is_selected: bool,
        styling: &InspectorStyling,
    ) -> Entity {
        let bg_color = if is_selected {
            Color::srgba(
                styling.highlight_color.0,
                styling.highlight_color.1,
                styling.highlight_color.2,
                0.3,
            )
        } else {
            Color::NONE
        };
        
        let entity = commands
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(24.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(styling.padding / 2.0)),
                    margin: UiRect::vertical(Val::Px(1.0)),
                    ..Default::default()
                },
                BackgroundColor(bg_color),
                EntityListItem {
                    entity_id,
                    display_name: display_name.to_string(),
                    is_selected,
                },
                InspectorMarker,
            ))
            .with_children(|parent| {
                // Entity name and ID
                parent.spawn((
                    Text::new(format!("• {} [{}]", display_name, entity_id.index())),
                    TextFont {
                        font_size: styling.font_size_normal,
                        ..Default::default()
                    },
                    TextColor(Color::srgba(
                        styling.text_color.0,
                        styling.text_color.1,
                        styling.text_color.2,
                        styling.text_color.3,
                    )),
                    InspectorMarker,
                ));
                
                // Component badges
                if !component_names.is_empty() {
                    parent.spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(4.0),
                            margin: UiRect::top(Val::Px(2.0)),
                            ..Default::default()
                        },
                        InspectorMarker,
                    )).with_children(|badges_parent| {
                        for component_name in component_names.iter().take(5) { // Limit to 5 badges
                            ComponentBadge::spawn(badges_parent, component_name, true, styling);
                        }
                        
                        if component_names.len() > 5 {
                            badges_parent.spawn((
                                Text::new(format!("+ {} more", component_names.len() - 5)),
                                TextFont {
                                    font_size: styling.font_size_small,
                                    ..Default::default()
                                },
                                TextColor(Color::srgba(0.7, 0.7, 0.7, 1.0)),
                                InspectorMarker,
                            ));
                        }
                    });
                }
            })
            .id();
            
        commands.entity(parent).add_child(entity);
        entity
    }
}

impl ComponentBadge {
    /// Spawn a component badge
    pub fn spawn(
        parent: &mut RelatedSpawnerCommands<'_, ChildOf>,
        component_name: &str,
        is_reflected: bool,
        styling: &InspectorStyling,
    ) {
        let badge_color = if is_reflected {
            Color::srgba(0.2, 0.6, 0.2, 0.8) // Green for reflected components
        } else {
            Color::srgba(0.6, 0.6, 0.2, 0.8) // Yellow for non-reflected
        };
        
        parent.spawn((
            Node {
                padding: UiRect::all(Val::Px(2.0)),
                margin: UiRect::all(Val::Px(1.0)),
                ..Default::default()
            },
            BackgroundColor(badge_color),
            ComponentBadge {
                component_name: component_name.to_string(),
                is_reflected,
            },
            InspectorMarker,
        )).with_children(|badge_parent| {
            badge_parent.spawn((
                Text::new(component_name),
                TextFont {
                    font_size: styling.font_size_small,
                    ..Default::default()
                },
                TextColor(Color::WHITE),
                InspectorMarker,
            ));
        });
    }
}

impl ResizeHandle {
    /// Spawn a resize handle between panels
    pub fn spawn(
        commands: &mut Commands,
        parent: Entity,
        target_panel: Entity,
        direction: ResizeDirection,
    ) -> Entity {
        let (width, height, cursor_style) = match direction {
            ResizeDirection::Horizontal => (Val::Px(4.0), Val::Percent(100.0), "col-resize"),
            ResizeDirection::Vertical => (Val::Percent(100.0), Val::Px(4.0), "row-resize"),
        };
        
        let entity = commands
            .spawn((
                Button,
                Node {
                    width,
                    height,
                    ..Default::default()
                },
                BackgroundColor(Color::srgba(0.25, 0.27, 0.31, 1.0)),
                ResizeHandle {
                    target_panel,
                    resize_direction: direction,
                    is_dragging: false,
                    last_cursor_pos: None,
                },
                InspectorMarker,
            ))
            .id();
            
        commands.entity(parent).add_child(entity);
        entity
    }
}

impl SearchBox {
    /// Spawn a search input box
    pub fn spawn(
        commands: &mut Commands,
        parent: Entity,
        placeholder: &str,
        styling: &InspectorStyling,
    ) -> Entity {
        let entity = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(32.0),
                    padding: UiRect::all(Val::Px(styling.padding)),
                    margin: UiRect::all(Val::Px(styling.margin)),
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 1.0)),
                SearchBox {
                    placeholder: placeholder.to_string(),
                    current_text: String::new(),
                },
                InspectorMarker,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(format!("Search: {}", placeholder)),
                    TextFont {
                        font_size: styling.font_size_normal,
                        ..Default::default()
                    },
                    TextColor(Color::srgba(0.7, 0.7, 0.7, 1.0)),
                    InspectorMarker,
                ));
            })
            .id();
            
        commands.entity(parent).add_child(entity);
        entity
    }
}

/// UI widget spawning utilities
pub struct WidgetSpawner;

impl WidgetSpawner {
    /// Create a titled section with content
    pub fn spawn_section(
        commands: &mut Commands,
        parent: Entity,
        title: &str,
        styling: &InspectorStyling,
    ) -> Entity {
        let section = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    margin: UiRect::all(Val::Px(styling.margin)),
                    ..Default::default()
                },
                InspectorMarker,
            ))
            .with_children(|parent| {
                // Section title
                parent.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(24.0),
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(styling.padding / 2.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgba(
                        styling.header_color.0,
                        styling.header_color.1,
                        styling.header_color.2,
                        styling.header_color.3,
                    )),
                    InspectorMarker,
                )).with_children(|title_parent| {
                    title_parent.spawn((
                        Text::new(title),
                        TextFont {
                            font_size: styling.font_size_header,
                            ..Default::default()
                        },
                        TextColor(Color::srgba(
                            styling.text_color.0,
                            styling.text_color.1,
                            styling.text_color.2,
                            styling.text_color.3,
                        )),
                        InspectorMarker,
                    ));
                });
            })
            .id();
            
        commands.entity(parent).add_child(section);
        section
    }
    
    /// Create a scrollable container
    pub fn spawn_scrollable_container(
        commands: &mut Commands,
        parent: Entity,
        styling: &InspectorStyling,
    ) -> Entity {
        let container = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip_y(),
                    ..Default::default()
                },
                BackgroundColor(Color::srgba(
                    styling.background_color.0,
                    styling.background_color.1,
                    styling.background_color.2,
                    styling.background_color.3,
                )),
                ScrollableContainer {
                    scroll_position: 0.0,
                    content_height: 0.0,
                },
                InspectorMarker,
            ))
            .id();
            
        commands.entity(parent).add_child(container);
        container
    }
}
