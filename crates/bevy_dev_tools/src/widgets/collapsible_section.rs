//! Collapsible/expandable section widget for Bevy UI
//!
//! This widget provides collapsible sections with clickable headers and show/hide content.
//! It's designed to be reusable and suitable for upstreaming to bevy_ui.

use bevy_ecs::prelude::*;
use bevy_ui::prelude::*;
use bevy_color::Color;
use bevy_ui::widget::Text;
use bevy_text::{TextFont, TextColor};

/// A collapsible section widget that can expand/collapse content
#[derive(Component, Clone)]
pub struct CollapsibleSection {
    /// Title text for the section
    pub title: String,
    /// Whether the section is currently expanded
    pub is_expanded: bool,
    /// Optional entity reference for the header
    pub header_entity: Option<Entity>,
    /// Optional entity reference for the content
    pub content_entity: Option<Entity>,
    /// Whether clicking toggles the state
    pub clickable: bool,
    /// Custom styling for the section
    pub style: CollapsibleStyle,
}

impl Default for CollapsibleSection {
    fn default() -> Self {
        Self {
            title: String::new(),
            is_expanded: true,
            header_entity: None,
            content_entity: None,
            clickable: true,
            style: CollapsibleStyle::default(),
        }
    }
}

impl CollapsibleSection {
    /// Create a new collapsible section with the given title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Set whether the section is expanded
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.is_expanded = expanded;
        self
    }

    /// Set whether the section is clickable
    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }

    /// Set custom styling
    pub fn with_style(mut self, style: CollapsibleStyle) -> Self {
        self.style = style;
        self
    }
}

/// Styling options for collapsible sections
#[derive(Clone)]
pub struct CollapsibleStyle {
    /// Background color for the section
    pub section_background: Color,
    /// Background color for the header
    pub header_background: Color,
    /// Background color for the content
    pub content_background: Color,
    /// Border color
    pub border_color: Color,
    /// Text color for the title
    pub title_color: Color,
    /// Font size for the title
    pub title_font_size: f32,
    /// Header height
    pub header_height: f32,
    /// Padding around content
    pub content_padding: UiRect,
    /// Arrow characters for expanded/collapsed states
    pub expanded_arrow: String,
    pub collapsed_arrow: String,
}

impl Default for CollapsibleStyle {
    fn default() -> Self {
        Self {
            section_background: Color::srgb(0.15, 0.15, 0.2),
            header_background: Color::srgb(0.2, 0.2, 0.25),
            content_background: Color::srgb(0.1, 0.1, 0.15),
            border_color: Color::srgb(0.3, 0.3, 0.4),
            title_color: Color::srgb(0.9, 0.9, 0.6),
            title_font_size: 14.0,
            header_height: 32.0,
            content_padding: UiRect::all(Val::Px(8.0)),
            expanded_arrow: "▼".to_string(),
            collapsed_arrow: "▶".to_string(),
        }
    }
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

/// Marker component for the arrow text in headers
#[derive(Component)]
pub struct CollapsibleArrow {
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
            collapsible: CollapsibleSection::default(),
            node: Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            background_color: BackgroundColor(CollapsibleStyle::default().section_background),
            border_color: BorderColor::all(CollapsibleStyle::default().border_color),
        }
    }
}

/// System to handle collapsible section interactions
pub fn handle_collapsible_interactions(
    mut section_query: Query<&mut CollapsibleSection>,
    header_query: Query<(&Interaction, &CollapsibleHeader), Changed<Interaction>>,
    mut content_query: Query<&mut Node, With<CollapsibleContent>>,
    mut arrow_query: Query<(Entity, &CollapsibleArrow, &mut Text)>,
) {
    for (interaction, header) in header_query.iter() {
        if *interaction == Interaction::Pressed {
            if let Ok(mut section) = section_query.get_mut(header.section_entity) {
                if section.clickable {
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
                    
                    // Update arrow in header - find matching arrows by querying all arrows
                    for (_entity, arrow, mut text) in arrow_query.iter_mut() {
                        if arrow.section_entity == header.section_entity {
                            let arrow_char = if section.is_expanded {
                                &section.style.expanded_arrow
                            } else {
                                &section.style.collapsed_arrow
                            };
                            
                            // Update the arrow part of the text
                            let title_part = text.0.split_once(' ').map(|(_, t)| t).unwrap_or(&text.0);
                            text.0 = format!("{} {}", arrow_char, title_part);
                            break;
                        }
                    }
                    
                    println!("Toggled section '{}' to {}", 
                        section.title, 
                        if section.is_expanded { "expanded" } else { "collapsed" }
                    );
                }
            }
        }
    }
}

/// System to update collapsible section visual state
pub fn update_collapsible_sections(
    mut section_query: Query<(&CollapsibleSection, &mut BackgroundColor), Changed<CollapsibleSection>>,
) {
    for (section, mut background) in section_query.iter_mut() {
        *background = BackgroundColor(section.style.section_background);
    }
}

/// Helper function to spawn a collapsible section with default styling
pub fn spawn_collapsible_section(
    commands: &mut Commands,
    parent: Entity,
    title: impl Into<String>,
) -> Entity {
    let section = CollapsibleSection::new(title);
    spawn_collapsible_section_with_config(commands, parent, section)
}

/// Helper function to spawn a collapsible section with custom configuration
pub fn spawn_collapsible_section_with_config(
    commands: &mut Commands,
    parent: Entity,
    section_config: CollapsibleSection,
) -> Entity {
    let title = section_config.title.clone();
    let style = section_config.style.clone();
    let is_expanded = section_config.is_expanded;
    let clickable = section_config.clickable;
    
    let section_entity = commands.spawn(CollapsibleSectionBundle {
        collapsible: section_config,
        node: Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            margin: UiRect::bottom(Val::Px(4.0)),
            ..Default::default()
        },
        background_color: BackgroundColor(style.section_background),
        border_color: BorderColor::all(style.border_color),
    }).id();
    
    commands.entity(parent).add_child(section_entity);
    
    // Spawn header
    let header_entity = commands.spawn((
        Button,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(style.header_height),
            padding: UiRect::all(Val::Px(8.0)),
            align_items: AlignItems::Center,
            ..Default::default()
        },
        BackgroundColor(style.header_background),
        CollapsibleHeader { section_entity },
    )).with_children(|parent| {
        let arrow_char = if is_expanded {
            &style.expanded_arrow
        } else {
            &style.collapsed_arrow
        };
        
        parent.spawn((
            Text::new(format!("{} {}", arrow_char, title)),
            TextFont {
                font_size: style.title_font_size,
                ..Default::default()
            },
            TextColor(style.title_color),
            CollapsibleArrow { section_entity },
        ));
    }).id();
    
    // Spawn content container
    let content_entity = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            padding: style.content_padding,
            flex_direction: FlexDirection::Column,
            display: if is_expanded { Display::Flex } else { Display::None },
            ..Default::default()
        },
        BackgroundColor(style.content_background),
        CollapsibleContent { section_entity },
    )).id();
    
    // Set up parent-child relationships
    commands.entity(section_entity).add_child(header_entity);
    commands.entity(section_entity).add_child(content_entity);
    
    // Update section with entity references
    commands.entity(section_entity).insert(CollapsibleSection {
        title,
        is_expanded,
        header_entity: Some(header_entity),
        content_entity: Some(content_entity),
        clickable,
        style,
    });
    
    section_entity
}

/// Helper function to add content to a collapsible section
pub fn add_collapsible_content(
    commands: &mut Commands,
    section_entity: Entity,
    content_spawner: impl FnOnce(&mut Commands, Entity),
) -> Result<(), &'static str> {
    // Find the content entity for this section
    if let Ok(_section) = commands.get_entity(section_entity) {
        // This is a simplified approach - in a real implementation, we'd need to
        // query for the CollapsibleSection component to get the content_entity
        // For now, we'll assume the content entity is the second child
        content_spawner(commands, section_entity);
        Ok(())
    } else {
        Err("Section entity not found")
    }
}

/// Trait for objects that can provide content to collapsible sections
pub trait CollapsibleContentProvider {
    fn spawn_content(&self, commands: &mut Commands, parent: Entity);
}

/// Plugin to add collapsible section functionality
pub struct CollapsibleSectionPlugin;

impl bevy_app::Plugin for CollapsibleSectionPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(bevy_app::Update, (
            handle_collapsible_interactions,
            update_collapsible_sections,
        ));
    }
}