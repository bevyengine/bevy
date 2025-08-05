//! Collapsible section widget - suitable for upstreaming to bevy_ui

use bevy_ecs::prelude::*;
use bevy_ui::prelude::*;
use bevy_color::Color;
use bevy_text::{TextFont, TextColor};
use bevy_log::info;

/// A collapsible section widget that can expand/collapse content
#[derive(Component)]
pub struct CollapsibleSection {
    pub title: String,
    pub is_expanded: bool,
    pub header_entity: Option<Entity>,
    pub content_entity: Option<Entity>,
}

/// Marker component for collapsible section headers (clickable)
#[derive(Component)]
pub struct CollapsibleHeader {
    pub section_entity: Entity,
}

/// Marker component for collapsible section content (shows/hides)
#[derive(Component)]
pub struct CollapsibleContent {
    pub section_entity: Entity,
}

/// Bundle for creating a collapsible section
#[derive(Bundle)]
pub struct CollapsibleSectionBundle {
    pub collapsible: CollapsibleSection,
    pub node: Node,
    pub background_color: BackgroundColor,
    pub border_color: BorderColor,
}

impl Default for CollapsibleSectionBundle {
    fn default() -> Self {
        Self {
            collapsible: CollapsibleSection {
                title: String::new(),
                is_expanded: true,
                header_entity: None,
                content_entity: None,
            },
            node: Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::srgb(0.15, 0.15, 0.2)),
            border_color: BorderColor::all(Color::srgb(0.3, 0.3, 0.4)),
        }
    }
}

/// System to handle collapsible section interactions
pub fn handle_collapsible_interactions(
    _commands: Commands,
    mut section_query: Query<&mut CollapsibleSection>,
    header_query: Query<(&Interaction, &CollapsibleHeader), Changed<Interaction>>,
    mut content_query: Query<&mut Node, With<CollapsibleContent>>,
) {
    for (interaction, header) in header_query.iter() {
        if *interaction == Interaction::Pressed {
            if let Ok(mut section) = section_query.get_mut(header.section_entity) {
                // Toggle expansion state
                section.is_expanded = !section.is_expanded;
                
                // Update content visibility
                if let Some(content_entity) = section.content_entity {
                    if let Ok(mut content_node) = content_query.get_mut(content_entity) {
                        content_node.display = if section.is_expanded {
                            Display::Flex
                        } else {
                            Display::None
                        };
                    }
                }
                
                // Update header arrow (would need access to text component)
                info!("Toggled section '{}' to {}", 
                    section.title, 
                    if section.is_expanded { "expanded" } else { "collapsed" }
                );
            }
        }
    }
}

/// Helper function to spawn a collapsible section
pub fn spawn_collapsible_section(
    commands: &mut Commands,
    parent: Entity,
    title: String,
) -> Entity {
    let section_entity = commands.spawn(CollapsibleSectionBundle {
        collapsible: CollapsibleSection {
            title: title.clone(),
            is_expanded: true,
            header_entity: None,
            content_entity: None,
        },
        ..Default::default()
    }).id();
    
    commands.entity(parent).add_child(section_entity);
    
    // Spawn header
    let header_entity = commands.spawn((
        Button,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(32.0),
            padding: UiRect::all(Val::Px(8.0)),
            align_items: AlignItems::Center,
            ..Default::default()
        },
        BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
        CollapsibleHeader { section_entity },
    )).with_children(|parent| {
        parent.spawn((
            Text::new(format!("▼ {}", title)),
            TextFont {
                font_size: 14.0,
                ..Default::default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.6)),
        ));
    }).id();
    
    // Spawn content container
    let content_entity = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(8.0)),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
        CollapsibleContent { section_entity },
    )).id();
    
    // Set up parent-child relationships
    commands.entity(section_entity).add_child(header_entity);
    commands.entity(section_entity).add_child(content_entity);
    
    // Update section with entity references
    commands.entity(section_entity).insert(CollapsibleSection {
        title,
        is_expanded: true,
        header_entity: Some(header_entity),
        content_entity: Some(content_entity),
    });
    
    section_entity
}