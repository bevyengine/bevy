#![expect(
    clippy::uninlined_format_args,
    reason = "More readable in debug context"
)]

//! Collapsible section widget - suitable for upstreaming to `bevy_ui`

use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_text::{TextColor, TextFont};
use bevy_ui::prelude::*;

/// A collapsible section widget that can expand/collapse content
#[derive(Component)]
pub struct CollapsibleSection {
    /// Display title for the collapsible section
    pub title: String,
    /// Whether the section is currently expanded or collapsed
    pub is_expanded: bool,
    /// Entity reference to the clickable header element
    pub header_entity: Option<Entity>,
    /// Entity reference to the collapsible content element
    pub content_entity: Option<Entity>,
}

/// Marker component for collapsible section headers (clickable)
#[derive(Component)]
pub struct CollapsibleHeader {
    /// Reference to the parent collapsible section entity
    pub section_entity: Entity,
}

/// Marker component for collapsible section content (shows/hides)
#[derive(Component)]
pub struct CollapsibleContent {
    /// Reference to the parent collapsible section entity
    pub section_entity: Entity,
}

/// Marker component for the text that shows the collapse/expand arrow
#[derive(Component)]
pub struct CollapsibleArrowText {
    /// Reference to the parent collapsible section entity
    pub section_entity: Entity,
    /// The text format template (without arrow) for updating
    pub text_template: String,
}

/// Bundle for creating a collapsible section
#[derive(Bundle)]
pub struct CollapsibleSectionBundle {
    /// The collapsible section component with state and references
    pub collapsible: CollapsibleSection,
    /// UI node layout properties for the section
    pub node: Node,
    /// Background color styling for the section
    pub background_color: BackgroundColor,
    /// Border color styling for the section
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
    mut arrow_text_query: Query<(&mut Text, &CollapsibleArrowText)>,
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

                // Update header arrow text
                for (mut arrow_text, arrow_marker) in arrow_text_query.iter_mut() {
                    if arrow_marker.section_entity == header.section_entity {
                        let arrow = if section.is_expanded { "-" } else { "+" };
                        arrow_text.0 = format!("{} {}", arrow, arrow_marker.text_template);
                    }
                }
            }
        }
    }
}

/// Helper function to spawn a collapsible section
pub fn spawn_collapsible_section(commands: &mut Commands, parent: Entity, title: String) -> Entity {
    let section_entity = commands
        .spawn(CollapsibleSectionBundle {
            collapsible: CollapsibleSection {
                title: title.clone(),
                is_expanded: true,
                header_entity: None,
                content_entity: None,
            },
            ..Default::default()
        })
        .id();

    commands.entity(parent).add_child(section_entity);

    // Spawn header
    let header_entity = commands
        .spawn((
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
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("- {}", title)),
                TextFont {
                    font_size: 14.0,
                    ..Default::default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.6)),
                CollapsibleArrowText {
                    section_entity,
                    text_template: title.clone(),
                },
            ));
        })
        .id();

    // Spawn content container
    let content_entity = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
            CollapsibleContent { section_entity },
        ))
        .id();

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
