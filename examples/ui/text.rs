//! This example illustrates how to create UI text and update it in a system.
//!
//! It displays the current FPS in the top left corner, as well as text that changes color
//! in the bottom right. For text within a scene, please see the text2d example.

use bevy::{
    color::palettes::css::GOLD,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (text_update_system, text_color_system))
        .run();
}

// A unit struct to help identify the FPS UI component, since there may be many Text components
#[derive(Component)]
struct FpsText;

// A unit struct to help identify the color-changing Text component
#[derive(Component)]
struct ColorText;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2dBundle::default());
    // Text with one section.
    commands.spawn((
        // Accepts a `String` or any type that converts into a `String`, such as `&str`
        TextNEW::new("hello\nbevy!"),
        TextStyle {
            // This font is loaded and will be used instead of the default font.
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 67.0,
            ..default()
        },
        // Set the justification of the Text
        TextBlock::new_with_justify(JustifyText::Center),
        // Set the style of the Node itself.
        Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(5.0),
            ..default()
        },
        ColorText,
    ));

    // Text with multiple sections
    commands
        .spawn((
            // Create a Text with multiple child spans.
            TextNEW::new("FPS: "),
            TextStyle {
                // This font is loaded and will be used instead of the default font.
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 42.0,
                ..default()
            },
        ))
        .with_child((
            TextSpan::default(),
            if cfg!(feature = "default_font") {
                TextStyle {
                    font_size: 33.0,
                    color: GOLD.into(),
                    // If no font is specified, the default font (a minimal subset of FiraMono) will be used.
                    ..default()
                }
            } else {
                // "default_font" feature is unavailable, load a font to use instead.
                TextStyle {
                    font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                    font_size: 33.0,
                    color: GOLD.into(),
                }
            },
            FpsText,
        ));

    #[cfg(feature = "default_font")]
    commands.spawn((
        // Here we are able to call the `From` method instead of creating a new `TextSection`.
        // This will use the default font (a minimal subset of FiraMono) and apply the default styling.
        TextNEW::new("From an &str into a Text with the default font!"),
        Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            left: Val::Px(15.0),
            ..default()
        },
    ));

    #[cfg(not(feature = "default_font"))]
    commands.spawn((
        TextNEW::new("Default font disabled"),
        TextStyle {
            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
            ..default()
        },
        Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            left: Val::Px(15.0),
            ..default()
        },
    ));
}

fn text_color_system(time: Res<Time>, mut query: Query<&mut TextStyle, With<ColorText>>) {
    for mut style in &mut query {
        let seconds = time.elapsed_seconds();

        // Update the color of the ColorText span.
        style.color = Color::srgb(
            ops::sin(1.25 * seconds) / 2.0 + 0.5,
            ops::sin(0.75 * seconds) / 2.0 + 0.5,
            ops::sin(0.50 * seconds) / 2.0 + 0.5,
        );
    }
}

fn text_update_system(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut TextSpan, With<FpsText>>,
) {
    for mut span in &mut query {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                // Update the value of the second section
                **span = format!("{value:.2}");
            }
        }
    }
}
