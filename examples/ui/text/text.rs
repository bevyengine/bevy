//! This example illustrates how to create UI text and update it in a system.
//!
//! It displays the current FPS in the top left corner, as well as text that changes color
//! in the bottom right. For text within a scene, please see the text2d example.

use bevy::{
    color::palettes::css::GOLD,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    text::FontSourceTemplate,
    text::{FontFeatureTag, FontFeatures, FontSize, Underline},
};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, (text_update_system, text_color_system));
    app.run();
}

// Marker struct to help identify the FPS UI component, since there may be many Text components
#[derive(Component, Default, Clone)]
struct FpsText;

// Marker struct to help identify the color-changing Text component
#[derive(Component, Default, Clone)]
struct AnimatedText;

fn setup(world: &mut World) -> Result {
    world.spawn_scene_list(bsn_list![
        Camera2d,
        text_with_one_section(),
        text_with_multiple_sections(),
        text_with_open_type_features(),
    ])?;

    #[cfg(feature = "default_font")]
    world.spawn_scene_list(bsn_list![default_font(),])?;

    #[cfg(not(feature = "default_font"))]
    world.spawn_scene_list(bsn_list![default_font_disabled()])?;
    Ok(())
}

fn text_with_one_section() -> impl Scene {
    bsn! {
        // Accepts a `String` or any type that converts into a `String`, such as `&str`
        Text::new("hello\nbevy!")
        Underline
        TextFont {
            // This font is loaded and will be used instead of the default font.
            font: FontSourceTemplate::Handle("fonts/FiraSans-Bold.ttf"),
            // The size of the text will be 20% of the viewport height.
            font_size: FontSize::Vh(20.0),
        }
        TextShadow::default()
        // Set the justification of the Text
        TextLayout::new_with_justify(Justify::Center)
        // Set the style of the Node itself.
        Node {
            position_type: PositionType::Absolute,
            bottom: px(5),
            right: px(5),
        }
        AnimatedText
    }
}

fn text_with_multiple_sections() -> impl Scene {
    bsn! {
        // Create a Text with multiple child spans.
        Text::new("FPS: ")
        TextFont {
            // This font is loaded and will be used instead of the default font.
            font: FontSourceTemplate::Handle("fonts/FiraSans-Bold.ttf"),
            font_size: FontSize::Px(42.0),
        }
        Children [
            (
                TextSpan
                Children [(
                    template(|_ctx| {
                        Ok(TextFont {
                            // If the "default_font" feature is unavailable, load a font to use instead.
                            #[cfg(not(feature = "default_font"))]
                            font: FontSource::Handle(_ctx.resource::<AssetServer>().load("fonts/FiraMono-Medium.ttf")),
                            font_size: FontSize::Px(33.0),
                            ..default()
                        })
                    })
                )]
                TextColor(Color::from(GOLD))
                FpsText
            )
        ]
    }
}

fn text_with_open_type_features() -> impl Scene {
    type TextRows = (&'static str, FontFeatureTag, &'static str);

    let text_rows: [TextRows; 7] = [
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

    fn title(row: TextRows) -> impl Scene {
        bsn! {
            TextSpan::new(row.0)
            TextFont {
                font: FontSourceTemplate::Handle("fonts/EBGaramond12-Regular.otf"),
                font_size: FontSize::Px(24.0),
            }
        }
    }

    fn text(row: TextRows) -> impl Scene {
        bsn! {
            TextSpan::new(format!("{0}\n", row.2))
            TextFont {
                font: FontSourceTemplate::Handle("fonts/EBGaramond12-Regular.otf"),
                font_size: FontSize::Px(24.0),
                font_features: { FontFeatures::builder().enable(row.1).build() },
            }
        }
    }

    bsn! {
        Node {
            margin: UiRect::all(px(12.0)),
            position_type: PositionType::Absolute,
            top: px(5.0),
            right: px(5.0),
        }
        Text::new("Opentype features:\n")
        TextFont {
            font: FontSourceTemplate::Handle("fonts/EBGaramond12-Regular.otf"),
            font_size: FontSize::Px(32.0),
        }
        Children [
            title(text_rows[0]), text(text_rows[0]),
            title(text_rows[1]), text(text_rows[1]),
            title(text_rows[2]), text(text_rows[2]),
            title(text_rows[3]), text(text_rows[3]),
            title(text_rows[4]), text(text_rows[4]),
            title(text_rows[5]), text(text_rows[5]),
            title(text_rows[6]), text(text_rows[6]),
        ]
    }
}

fn default_font() -> impl Scene {
    bsn! {
        // Here we are able to call the `From` method instead of creating a new `TextSection`.
        // This will use the default font (a minimal subset of FiraMono) and apply the default styling.
        Text::new("From an &str into a Text with the default font!")
        Node {
            position_type: PositionType::Absolute,
            bottom: px(5),
            left: px(15),
        }
    }
}

#[expect(dead_code, reason = "demonstration purpose")]
fn default_font_disabled() -> impl Scene {
    bsn! {
        Text::new("Default font disabled")
        TextFont {
            font: FontSourceTemplate::Handle("fonts/FiraMono-Medium.ttf"),
        }
        Node {
            position_type: PositionType::Absolute,
            bottom: px(5),
            left: px(15),
        }
    }
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
        //t.set_changed();
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
