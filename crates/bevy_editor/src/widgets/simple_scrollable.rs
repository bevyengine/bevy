//! Simple scrollable container widget for bevy_feathers extraction
//!
//! This module provides a basic scrollable container with mouse wheel support.
//! Designed to be extracted to the bevy_feathers UI library.
//!
//! # Features
//! - Mouse wheel scrolling with configurable sensitivity  
//! - Automatic overflow handling
//! - Minimal dependencies on core Bevy
//! - Plugin-based architecture
//!
//! # Usage
//! ```rust,no_run
//! use bevy::prelude::*;
//! use bevy_editor::widgets::ScrollableContainer;
//! use bevy_editor::widgets::simple_scrollable::ScrollableContainerPlugin;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(ScrollableContainerPlugin)
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn((
//!         Node {
//!             width: Val::Percent(100.0),
//!             height: Val::Px(300.0),
//!             overflow: Overflow::clip(),
//!             ..default()
//!         },
//!         ScrollableContainer {
//!             scroll_offset: 0.0,
//!             max_scroll: 1000.0,
//!             scroll_sensitivity: 10.0,
//!         },
//!     ));
//! }
//! ```

use bevy::prelude::*;

/// Basic scrollable container widget
#[derive(Component, Default)]
pub struct ScrollableContainer {
    pub scroll_offset: f32,
    pub max_scroll: f32,
    pub scroll_sensitivity: f32,
}

impl ScrollableContainer {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0.0,
            max_scroll: 0.0,
            scroll_sensitivity: 15.0,
        }
    }
}

/// Plugin for scrollable container functionality
pub struct ScrollableContainerPlugin;

impl Plugin for ScrollableContainerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_scroll_input);
    }
}

/// Basic scroll handling system
fn handle_scroll_input(
    mut scroll_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut scrollable_query: Query<&mut ScrollableContainer>,
) {
    for scroll_event in scroll_events.read() {
        // Simple scroll handling - apply to all scrollable containers for now
        for mut scrollable in &mut scrollable_query {
            let scroll_delta = scroll_event.y * scrollable.scroll_sensitivity;
            scrollable.scroll_offset = (scrollable.scroll_offset - scroll_delta)
                .clamp(-scrollable.max_scroll, 0.0);
        }
    }
}

/// Helper function to spawn a basic scrollable container
pub fn spawn_scrollable_container(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            ScrollableContainer::new(),
            Node {
                overflow: bevy::ui::Overflow::clip_y(),
                flex_direction: bevy::ui::FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .id()
}
