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
        .add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, (text_update_system, text_color_system))
        .run();
}

// Marker struct to help identify the FPS UI component, since there may be many Text components
#[derive(Component)]
struct FpsText;

// Marker struct to help identify the color-changing Text component
#[derive(Component)]
struct AnimatedText;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2d);
    // Text with one section
    commands.spawn((
        // Accepts a `String` or any type that converts into a `String`, such as `&str`
        Text::new("hello\nbevy!"),
        TextFont {
            // This font is loaded and will be used instead of the default font.
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 67.0,
            ..default()
        },
        TextShadow::default(),
        // Set the justification of the Text
        TextLayout::new_with_justify(Justify::Center),
        // Set the style of the Node itself.
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(5.0),
            ..default()
        },
        AnimatedText,
    ));

    // Text with multiple sections
    commands
        .spawn((
            // Create a Text with multiple child spans.
            Text::new("FPS: "),
            TextFont {
                // This font is loaded and will be used instead of the default font.
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 42.0,
                ..default()
            },
        ))
        .with_child((
            TextSpan::default(),
            if cfg!(feature = "default_font") {
                (
                    TextFont {
                        font_size: 33.0,
                        // If no font is specified, the default font (a minimal subset of FiraMono) will be used.
                        ..default()
                    },
                    TextColor(GOLD.into()),
                )
            } else {
                (
                    // "default_font" feature is unavailable, load a font to use instead.
                    TextFont {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: 33.0,
                        ..Default::default()
                    },
                    TextColor(GOLD.into()),
                )
            },
            FpsText,
        ));

    #[cfg(feature = "default_font")]
    commands.spawn((
        // Here we are able to call the `From` method instead of creating a new `TextSection`.
        // This will use the default font (a minimal subset of FiraMono) and apply the default styling.
        Text::new("From an &str into a Text with the default font!"),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            left: Val::Px(15.0),
            ..default()
        },
    ));

    #[cfg(not(feature = "default_font"))]
    commands.spawn((
        Text::new("Default font disabled"),
        TextFont {
            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            left: Val::Px(15.0),
            ..default()
        },
    ));
}

fn text_color_system(time: Res<Time>, mut query: Query<&mut TextColor, With<AnimatedText>>) {
    for mut text_color in &mut query {
        let seconds = time.elapsed_secs();

        // Update the color of the ColorText span.
        text_color.0 = Color::srgb(
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
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
            && let Some(value) = fps.smoothed()
        {
            // Update the value of the second section
            **span = format!("{value:.2}");
        }
    }
}
