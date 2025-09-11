//! This example illustrates how to create UI text and update it in a system.
//!
//! It displays the current FPS in the top left corner, as well as text that changes color
//! in the bottom right. For text within a scene, please see the text2d example.

use bevy::{
    color::palettes::css::GOLD,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    text::{FontFace, FontSize},
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (text_update_system, text_color_system), //, embiggen_font_system),
        )
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

    let font_67_bold = commands
        .spawn((
            FontFace(asset_server.load("fonts/FiraSans-Bold.ttf")),
            FontSize(67.),
        ))
        .id();

    let font_42_bold = commands
        .spawn((
            FontFace(asset_server.load("fonts/FiraSans-Bold.ttf")),
            FontSize(42.),
        ))
        .id();

    let font_33 = commands
        .spawn((
            FontFace(asset_server.load("fonts/FiraMono-Medium.ttf")),
            FontSize(33.),
        ))
        .id();

    // Text with one section
    commands.spawn((
        // Accepts a `String` or any type that converts into a `String`, such as `&str`
        Text::new("hello\nbevy!"),
        TextFont(font_67_bold),
        TextShadow::default(),
        // Set the justification of the Text
        TextLayout::new_with_justify(Justify::Center),
        // Set the style of the Node itself.
        Node {
            position_type: PositionType::Absolute,
            bottom: px(5),
            right: px(5),
            ..default()
        },
        AnimatedText,
    ));

    // Text with multiple sections
    commands
        .spawn((
            // Create a Text with multiple child spans.
            Text::new("FPS: "),
            TextFont(font_42_bold),
        ))
        .with_child((
            TextSpan::default(),
            (
                // "default_font" feature is unavailable, load a font to use instead.
                TextFont(font_33),
                TextColor(GOLD.into()),
            ),
            FpsText,
        ));

    #[cfg(feature = "default_font")]
    commands.spawn((
        // Here we are able to call the `From` method instead of creating a new `TextSection`.
        // This will use the default font (a minimal subset of FiraMono) and apply the default styling.
        Text::new("From an &str into a Text with the default font!"),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(5),
            left: px(15),
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
            bottom: px(5),
            left: px(15),
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

// fn embiggen_font_system(mut query: Query<&mut TextFont>) {
//     for mut font in query.iter_mut() {
//         font.font_size *= 1.01;
//     }
// }
