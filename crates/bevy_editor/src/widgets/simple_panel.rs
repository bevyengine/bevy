//! Basic panel container widget for bevy_feathers extraction
//!
//! This module provides a simple panel widget with title and configurable dimensions.
//! Designed to be modular and reusable across different applications.
//!
//! # Features
//! - Titled panels with header styling
//! - Configurable minimum dimensions
//! - Consistent theme integration
//! - Minimal external dependencies
//!
//! # Usage
//! ```rust
//! use bevy::prelude::*;
//! use bevy_editor::widgets::{BasicPanel, spawn_basic_panel};
//!
//! fn setup(mut commands: Commands) {
//!     // Quick panel creation
//!     spawn_basic_panel(&mut commands, "My Panel");
//!     
//!     // Manual panel creation with custom settings
//!     commands.spawn((
//!         BasicPanel {
//!             title: "Custom Panel".to_string(),
//!             min_width: 200.0,
//!             min_height: 100.0,
//!         },
//!         Node {
//!             flex_direction: FlexDirection::Column,
//!             width: Val::Px(300.0),
//!             height: Val::Px(400.0),
//!             ..default()
//!         },
//!     ));
//! }
//! ```

use bevy::prelude::*;

/// Basic panel container
#[derive(Component, Clone)]
pub struct BasicPanel {
    pub title: String,
    pub min_width: f32,
    pub min_height: f32,
}

impl BasicPanel {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            min_width: 100.0,
            min_height: 100.0,
        }
    }
}

/// Helper function to spawn a basic panel
pub fn spawn_basic_panel(
    commands: &mut Commands,
    title: impl Into<String>,
) -> Entity {
    let title = title.into();
    
    commands
        .spawn((
            BasicPanel::new(title.clone()),
            Node {
                flex_direction: bevy::ui::FlexDirection::Column,
                border: bevy::ui::UiRect::all(bevy::ui::Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
        ))
        .with_children(|parent| {
            // Header
            parent.spawn((
                Text::new(title),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                Node {
                    height: bevy::ui::Val::Px(30.0),
                    padding: bevy::ui::UiRect::all(bevy::ui::Val::Px(8.0)),
                    align_items: bevy::ui::AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            ));
        })
        .id()
}
