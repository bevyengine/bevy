//! This example demonstrates how to hotpatch systems.
//!
//! It needs to be run with the dioxus CLI:
//! ```sh
//! dx serve --hot-patch --example hotpatching_systems --features hotpatching
//! ```
//!
//! You can change the text in the `update_text` system, or the color in the
//! `on_click` system, and those changes will be hotpatched into the running
//! application.

use bevy::{color::palettes, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_text)
        .run();
}

fn update_text(mut tex: Single<&mut Text>) {
    **text = "before".to_string();
}

fn on_click(_click: Trigger<Pointer<Click>>, mut color: Single<&mut TextColor>) {
    color.0 = palettes::tailwind::RED_600.into();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            children![(
                Text::default(),
                TextFont {
                    font_size: 100.0,
                    ..default()
                },
            )],
        ))
        .observe(on_click);
}
