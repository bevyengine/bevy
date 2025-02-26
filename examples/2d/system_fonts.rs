//! Demonstrates searching for a system font at runtime.
//!
//! This example cycles through all available fonts, showing their name in said
//! font.
//! Pressing the spacebar key will select the next font.

use bevy::{platform_support::collections::HashSet, prelude::*, text::FontLibrary};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

/// Marker for the text we will update.
#[derive(Component)]
struct FontName;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text2d::new("Press Space to change the font"),
        TextFont {
            // We load a fall-back font, since we don't know that every platform
            // even has system fonts to use.
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 50.0,
            ..default()
        },
        FontName,
    ));
}

fn update(
    // FontLibrary provides access to all fonts loaded by the cosmic_text backend.
    // With the `system_font` feature enabled, this includes fonts installed on
    // the end-user's device.
    mut fonts: FontLibrary,
    mut query: Single<(&mut Text2d, &mut TextFont), With<FontName>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    // Want to make sure we don't show the same font twice
    mut used: Local<HashSet<Box<str>>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        let mut name = default();

        // The primary way to find a font is through FontLibrary::find,
        // which iterates over all loaded fonts, and returns the first that
        // satisfies the provided predicate.
        // In this example, we're just looking for any font we haven't already
        // shown.

        let Some(font) = fonts.find(|font| {
            let family_name = font.families[0].0.as_str();

            if used.contains(family_name) {
                return false;
            }

            used.insert(Box::from(family_name));
            name = String::from(family_name);
            true
        }) else {
            *query.0 = Text2d::new("No more fonts to show!");
            return;
        };

        *query.0 = Text2d::new(name);
        query.1.font = font;
    }
}
