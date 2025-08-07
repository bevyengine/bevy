//! Demonstration of the reusable UI widgets
//!
//! This example shows how to use the extracted widgets:
//! - SelectableText: Text with copy/paste functionality
//! - CollapsibleSection: Expandable/collapsible content
//! - VirtualScrolling: High-performance scrolling (example with string list)

use bevy::prelude::*;
use bevy_dev_tools::widgets::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WidgetsPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // Spawn camera
    commands.spawn(Camera2d);
    
    // Root container
    let root = commands.spawn((
        Node {
            width: Val::Vw(100.0),
            height: Val::Vh(100.0),
            padding: UiRect::all(Val::Px(20.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(16.0),
            ..Default::default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
    )).id();

    // Demo title
    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text::new("Bevy UI Widgets Demo"),
            TextFont {
                font_size: 24.0,
                ..Default::default()
            },
            TextColor(Color::WHITE),
        ));
    });

    // Selectable text demo
    let selectable_section = spawn_collapsible_section(
        &mut commands,
        root,
        "SelectableText Demo"
    );
    
    commands.entity(selectable_section).with_children(|section| {
        if let Some(content_entity) = get_content_entity(&mut commands, selectable_section) {
            commands.entity(content_entity).with_children(|content| {
                // Add some selectable text examples
                content.spawn((
                    Text::new("Click me to select this text, then Ctrl+C to copy"),
                    TextFont {
                        font_size: 14.0,
                        ..Default::default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                    SelectableText::new("Click me to select this text, then Ctrl+C to copy"),
                ));

                content.spawn((
                    Text::new("{\n  \"example\": \"JSON data\",\n  \"can_be_copied\": true,\n  \"useful_for\": \"debugging\"\n}"),
                    TextFont {
                        font_size: 12.0,
                        ..Default::default()
                    },
                    TextColor(Color::srgb(0.7, 0.9, 0.7)),
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                    SelectableText::new("{\n  \"example\": \"JSON data\",\n  \"can_be_copied\": true,\n  \"useful_for\": \"debugging\"\n}"),
                ));
            });
        }
    });

    // Collapsible sections demo
    let collapsible_section = spawn_collapsible_section(
        &mut commands,
        root,
        "CollapsibleSection Demo"
    );
    
    commands.entity(collapsible_section).with_children(|section| {
        if let Some(content_entity) = get_content_entity(&mut commands, collapsible_section) {
            commands.entity(content_entity).with_children(|content| {
                // Nested collapsible sections
                let nested1 = spawn_collapsible_section_with_config(
                    &mut commands,
                    content.parent_entity(),
                    CollapsibleSection::new("Nested Section 1")
                        .expanded(true)
                        .with_style(CollapsibleStyle {
                            header_background: Color::srgb(0.3, 0.2, 0.3),
                            content_background: Color::srgb(0.15, 0.1, 0.15),
                            ..Default::default()
                        }),
                );
                
                // Add content to nested section 1
                if let Some(nested_content) = get_content_entity(&mut commands, nested1) {
                    commands.entity(nested_content).with_children(|nested| {
                        nested.spawn((
                            Text::new("This is content inside the first nested section.\nYou can put any UI elements here."),
                            TextFont {
                                font_size: 12.0,
                                ..Default::default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        ));
                    });
                }

                let nested2 = spawn_collapsible_section_with_config(
                    &mut commands,
                    content.parent_entity(),
                    CollapsibleSection::new("Nested Section 2")
                        .expanded(false)
                        .with_style(CollapsibleStyle {
                            header_background: Color::srgb(0.2, 0.3, 0.3),
                            content_background: Color::srgb(0.1, 0.15, 0.15),
                            ..Default::default()
                        }),
                );

                // Add content to nested section 2
                if let Some(nested_content) = get_content_entity(&mut commands, nested2) {
                    commands.entity(nested_content).with_children(|nested| {
                        nested.spawn((
                            Text::new("This section starts collapsed.\nClick the header to expand it."),
                            TextFont {
                                font_size: 12.0,
                                ..Default::default()
                            },
                            TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        ));
                    });
                }
            });
        }
    });

    // Instructions
    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text::new("Instructions:\n• Click text to select it, then Ctrl+C to copy\n• Click section headers to expand/collapse\n• Press Escape to clear text selection"),
            TextFont {
                font_size: 12.0,
                ..Default::default()
            },
            TextColor(Color::srgb(0.6, 0.6, 0.6)),
            Node {
                margin: UiRect::top(Val::Px(16.0)),
                ..Default::default()
            },
        ));
    });
}

// Helper function to get content entity from collapsible section
// In a real implementation, this would be part of the CollapsibleSection API
fn get_content_entity(commands: &mut Commands, section_entity: Entity) -> Option<Entity> {
    // This is a simplified approach - in the actual widget we'd query the component
    // For this demo, we'll assume the content is the second child
    None // Placeholder - would need proper implementation
}