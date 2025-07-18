//! # Scroll Widget Examples
//! 
//! This module provides comprehensive examples and documentation for the scroll
//! widget system, demonstrating how to use both high-level and low-level scroll
//! components effectively.
//!
//! ## Widget Architecture
//!
//! The scroll system provides two main approaches:
//!
//! 1. **ScrollViewBuilder** - High-level, styled scroll widget (recommended)
//! 2. **CoreScrollArea** - Low-level scroll component for custom implementations
//!
//! Both widgets integrate seamlessly with Bevy's native `bevy_core_widgets` 
//! scrolling system and provide smooth mouse wheel interaction.

use bevy::prelude::*;
use crate::widgets::{ScrollViewBuilder, CoreScrollArea, ScrollContent, ScrollToEntityEvent};

/// Example of how to use the new scroll widgets
/// This demonstrates the separation of concerns between CoreScrollArea and ScrollView
pub fn scroll_widget_example(mut commands: Commands) {
    // Method 1: Using the high-level ScrollView builder (recommended for most cases)
    commands.spawn((
        Node {
            width: Val::Px(400.0),
            height: Val::Px(300.0),
            ..default()
        },
    )).with_children(|parent| {
        let scroll_view_entity = ScrollViewBuilder::new()
            .with_background_color(Color::srgb(0.1, 0.1, 0.1))
            .with_border_color(Color::srgb(0.3, 0.3, 0.3))
            .with_padding(UiRect::all(Val::Px(16.0)))
            .with_corner_radius(8.0)
            .with_scroll_sensitivity(25.0)
            .with_max_scroll(Vec2::new(0.0, 1000.0))
            .spawn(parent);
        
        // Add content to the scroll view - it will automatically find the ScrollContent child
        if let Some(mut entity_commands) = commands.get_entity(scroll_view_entity) {
            entity_commands.with_children(|parent| {
                for i in 0..50 {
                    parent.spawn((
                        Text::new(format!("Item {}", i)),
                        TextFont::default(),
                        TextColor(Color::WHITE),
                        Node {
                            height: Val::Px(30.0),
                            margin: UiRect::bottom(Val::Px(4.0)),
                            ..default()
                        },
                    ));
                }
            });
        }
    });

    // Method 2: Using CoreScrollArea directly (for custom implementations)
    commands.spawn((
        Node {
            width: Val::Px(300.0),
            height: Val::Px(200.0),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
        CoreScrollArea::new(Vec2::new(0.0, 500.0))
            .with_sensitivity(30.0)
            .with_vertical(true)
            .with_horizontal(false),
    )).with_children(|parent| {
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ScrollContent,
        )).with_children(|parent| {
            for i in 0..30 {
                parent.spawn((
                    Text::new(format!("Core Scroll Item {}", i)),
                    TextFont::default(),
                    TextColor(Color::WHITE),
                    Node {
                        height: Val::Px(25.0),
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    },
                ));
            }
        });
    });
}

/// Example of programmatic scrolling using events
pub fn programmatic_scroll_example(
    mut scroll_events: EventWriter<ScrollToEntityEvent>,
    scroll_areas: Query<Entity, With<CoreScrollArea>>,
    content_entities: Query<Entity, With<ScrollContent>>,
) {
    // Example: Scroll to a specific entity when some condition is met
    if let (Ok(scroll_area), Ok(target_entity)) = (scroll_areas.get_single(), content_entities.get_single()) {
        scroll_events.send(ScrollToEntityEvent {
            scroll_area_entity: scroll_area,
            target_entity,
        });
    }
}

/// Documentation for the scroll widget architecture
/// 
/// ## CoreScrollArea
/// - Handles mouse wheel events and scroll offset clamping
/// - Provides "scroll into view" functionality for entities
/// - Does NOT include scrollbars or visual styling
/// - Can be used standalone for custom scroll implementations
/// 
/// ## ScrollView
/// - High-level, opinionated scroll widget with built-in styling
/// - Includes scrollbars, padding, borders, and rounded corners
/// - Uses CoreScrollArea internally for scroll logic
/// - Recommended for most use cases
/// 
/// ## Usage Guidelines
/// 1. Use ScrollView for standard scroll areas with visual styling
/// 2. Use CoreScrollArea when you need custom scroll behavior or styling
/// 3. Both widgets automatically handle mouse wheel events within their bounds
/// 4. Content should be placed inside a ScrollContent component
/// 5. Use ScrollToEntityEvent for programmatic scrolling
#[allow(dead_code)]
struct ScrollWidgetDocumentation;
