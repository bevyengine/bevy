//! This example illustrates how to create UI text and update it in a system.
//!
//! It displays the current FPS in the top left corner, as well as a text effect showcase
//! in the bottom right. For text within a scene, please see the text2d example.

use bevy::{
    color::palettes::css::GOLD,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    text::{FontFeatureTag, FontFeatures, FontSize},
};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, (text_update_system, text_alpha_system));
    app.run();
}

// Marker struct to help identify the FPS UI component, since there may be many Text components
#[derive(Component)]
struct FpsText;

// Marker struct to help identify the alpha-animated Text component
#[derive(Component)]
struct AnimatedText;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let showcase_font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let showcase_text_font = TextFont {
        font: showcase_font.clone().into(),
        font_size: FontSize::Px(48.0),
        ..default()
    };

    // UI camera
    commands.spawn(Camera2d);
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: px(24),
            right: px(24),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::End,
            row_gap: px(10),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new("Text effects"),
                TextFont {
                    font: showcase_font.clone().into(),
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(GOLD.into()),
            ));
            parent.spawn((
                Text::new("Shadow only"),
                showcase_text_font.clone(),
                TextColor(Color::WHITE),
                TextShadow {
                    offset: Vec2::splat(8.0),
                    color: Color::BLACK.with_alpha(0.85),
                },
                TextLayout::new_with_justify(Justify::Right),
            ));
            parent.spawn((
                Text::new("Outline only"),
                showcase_text_font.clone(),
                TextColor(Color::srgb(0.98, 0.94, 0.83)),
                TextOutline {
                    color: Color::srgb(0.22, 0.09, 0.04),
                    width: 2.0,
                },
                TextLayout::new_with_justify(Justify::Right),
            ));
            parent.spawn((
                Text::new("Shadow + outline"),
                showcase_text_font.clone(),
                TextColor(Color::srgb(0.92, 0.97, 1.0)),
                TextShadow {
                    offset: Vec2::splat(8.0),
                    color: Color::BLACK.with_alpha(0.85),
                },
                TextOutline {
                    color: Color::srgb(0.12, 0.19, 0.35),
                    width: 2.0,
                },
                TextLayout::new_with_justify(Justify::Right),
            ));
            parent.spawn((
                Text::new("Animated alpha"),
                showcase_text_font,
                TextColor(Color::srgb(1.0, 0.95, 0.8)),
                TextShadow {
                    offset: Vec2::splat(8.0),
                    color: Color::BLACK.with_alpha(0.85),
                },
                TextOutline {
                    color: Color::srgb(0.25, 0.09, 0.02),
                    width: 2.0,
                },
                TextLayout::new_with_justify(Justify::Right),
                AnimatedText,
            ));
        });

    // Text with multiple sections
    commands
        .spawn((
            // Create a Text with multiple child spans.
            Text::new("FPS: "),
            TextFont {
                // This font is loaded and will be used instead of the default font.
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: FontSize::Px(42.0),
                ..default()
            },
        ))
        .with_child((
            TextSpan::default(),
            (
                TextFont {
                    // If the "default_font" feature is unavailable, load a font to use instead.
                    #[cfg(not(feature = "default_font"))]
                    font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                    font_size: FontSize::Px(33.0),
                    ..Default::default()
                },
                TextColor(GOLD.into()),
            ),
            FpsText,
        ));

    // Text with OpenType features
    let opentype_font_handle: FontSource =
        asset_server.load("fonts/EBGaramond12-Regular.otf").into();
    commands
        .spawn((
            Node {
                margin: UiRect::all(px(12.0)),
                position_type: PositionType::Absolute,
                top: px(5.0),
                right: px(5.0),
                ..default()
            },
            Text::new("Opentype features:\n"),
            TextFont {
                font: opentype_font_handle.clone(),
                font_size: FontSize::Px(32.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            let text_rows = [
                ("Smallcaps: ", FontFeatureTag::SMALL_CAPS, "Hello World"),
                (
                    "Ligatures: ",
                    FontFeatureTag::STANDARD_LIGATURES,
                    "fi fl ff ffi ffl",
                ),
                ("Fractions: ", FontFeatureTag::FRACTIONS, "12/134"),
                ("Superscript: ", FontFeatureTag::SUPERSCRIPT, "Up here!"),
                ("Subscript: ", FontFeatureTag::SUBSCRIPT, "Down here!"),
                (
                    "Oldstyle figures: ",
                    FontFeatureTag::OLDSTYLE_FIGURES,
                    "1234567890",
                ),
                (
                    "Lining figures: ",
                    FontFeatureTag::LINING_FIGURES,
                    "1234567890",
                ),
            ];

            for (title, feature, text) in text_rows {
                parent.spawn((
                    TextSpan::new(title),
                    TextFont {
                        font: opentype_font_handle.clone(),
                        font_size: FontSize::Px(24.0),
                        ..default()
                    },
                ));
                parent.spawn((
                    TextSpan::new(format!("{text}\n")),
                    TextFont {
                        font: opentype_font_handle.clone(),
                        font_size: FontSize::Px(24.0),
                        font_features: FontFeatures::builder().enable(feature).build(),
                        ..default()
                    },
                ));
            }
        });

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

fn text_alpha_system(time: Res<Time>, mut query: Query<&mut TextColor, With<AnimatedText>>) {
    for mut text_color in &mut query {
        let alpha = ops::sin(1.5 * time.elapsed_secs()) * 0.35 + 0.65;
        text_color.0 = Color::srgb(1.0, 0.95, 0.8).with_alpha(alpha);
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
