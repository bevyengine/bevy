//! Shows text rendering with moving, rotating and scaling text.
//!
//! Note that this uses [`Text2d`] to display text alongside your other entities in a 2D scene.
//!
//! For an example on how to render text as part of a user interface, independent from the world
//! viewport, you may want to look at `showcase/contributors.rs` or `ui/text.rs`.

use bevy::{
    color::palettes::css::*,
    math::ops,
    prelude::*,
    sprite::{Anchor, Text2dShadow},
    text::{FontSmoothing, FontSourceTemplate, LineBreak, TextBounds},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, scene.spawn())
        .add_systems(
            Update,
            (animate_translation, animate_rotation, animate_scale),
        )
        .run();
}

#[derive(Component, Clone, Default)]
struct AnimateTranslation;

#[derive(Component, Clone, Default)]
struct AnimateRotation;

#[derive(Component, Clone, Default)]
struct AnimateScale;

// font, font_size, text_justification, text_background_color
type ChangingProps = (&'static str, FontSize, Justify, Color);

// font, font_size, text_shadow_color
type TextWrappingProps = (&'static str, FontSize, Color);

// font, font_size
type OtherProps = (&'static str, FontSize);

fn scene() -> impl SceneList {
    let changing_props = (
        "fonts/FiraSans-Bold.ttf",
        FontSize::Px(50.0),
        Justify::Center,
        Color::BLACK.with_alpha(0.5),
    );

    let text_wrapping_props = (
        changing_props.0,
        FontSize::Px(35.0),
        Color::srgb(0.25, 0.25, 0.55).darker(0.05),
    );

    let other_props = (text_wrapping_props.0, text_wrapping_props.1);

    bsn_list![
        Camera2d,
        demonstrate_changing_translation(changing_props),
        demonstrate_changing_rotation(changing_props),
        demonstrate_changing_scale(changing_props),
        demonstrate_text_wrapping_with_unicode_linebreaks(text_wrapping_props),
        demonstrate_text_wrapping_with_any_character_linebreaks(text_wrapping_props),
        demonstrate_font_smoothing_off(other_props),
        demonstrate_anchors(other_props),
    ]
}

fn demonstrate_changing_translation(
    (font, font_size, text_justification, text_background_color): ChangingProps,
) -> impl Scene {
    bsn![
        Text2d::new(" translation ")
        TextFont {
            font: FontSourceTemplate::Handle(font),
            font_size: font_size,
        }
        TextLayout::new_with_justify(text_justification)
        TextBackgroundColor(text_background_color)
        Text2dShadow::default()
        AnimateTranslation
    ]
}

fn demonstrate_changing_rotation(
    (font, font_size, text_justification, text_background_color): ChangingProps,
) -> impl Scene {
    bsn![
        Text2d::new(" rotation ")
        TextFont {
            font: FontSourceTemplate::Handle(font),
            font_size: font_size,
        }
        TextLayout::new_with_justify(text_justification)
        TextBackgroundColor(text_background_color)
        Text2dShadow::default()
        AnimateRotation
    ]
}

fn demonstrate_changing_scale(
    (font, font_size, text_justification, text_background_color): ChangingProps,
) -> impl Scene {
    bsn![
        Text2d::new(" scale ")
        TextFont {
            font: FontSourceTemplate::Handle(font),
            font_size: font_size,
        }
        TextLayout::new_with_justify(text_justification)
        Transform::from_translation(Vec3::new(400.0, 0.0, 0.0))
        TextBackgroundColor(text_background_color)
        Text2dShadow::default()
        AnimateScale
    ]
}

fn demonstrate_text_wrapping_with_unicode_linebreaks(
    (font, font_size, text_shadow_color): TextWrappingProps,
) -> impl Scene {
    let box_size = Vec2::new(300.0, 200.0);
    let box_position = Vec2::new(0.0, -250.0);

    bsn![
        Sprite {
            color: Color::srgb(0.25, 0.25, 0.55),
            custom_size: box_size,
        }
        Transform::from_translation(box_position.extend(0.0))
        Children[(
            Text2d::new("this text wraps in the box\n(Unicode linebreaks)")
            TextFont {
                font: FontSourceTemplate::Handle(font),
                font_size: font_size,
            }
            TextLayout::new(Justify::Left, LineBreak::WordBoundary)
            // Wrap text in the rectangle
            TextBounds::from(box_size)
            // Ensure the text is drawn on top of the box
            Transform::from_translation(Vec3::Z)
            // Add a shadow to the text
            Text2dShadow {
                color: text_shadow_color,
            }
            Underline
        )]
    ]
}

fn demonstrate_text_wrapping_with_any_character_linebreaks(
    (font, font_size, text_shadow_color): TextWrappingProps,
) -> impl Scene {
    let other_box_size = Vec2::new(300.0, 200.0);
    let other_box_position = Vec2::new(320.0, -250.0);

    bsn![
        Sprite {
            color: Color::srgb(0.25, 0.25, 0.55),
            custom_size: other_box_size,
        }
        Transform::from_translation(other_box_position.extend(0.0))
        Children[(
            Text2d::new("this text wraps in the box\n(AnyCharacter linebreaks)")
            TextFont {
                font: FontSourceTemplate::Handle(font),
                font_size: font_size,
            }
            TextLayout::new(Justify::Left, LineBreak::AnyCharacter)
            // Wrap text in the rectangle
            TextBounds::from(other_box_size)
            // Ensure the text is drawn on top of the box
            Transform::from_translation(Vec3::Z)
            // Add a shadow to the text
            Text2dShadow {
                color: text_shadow_color,
            }
        )]
    ]
}

fn demonstrate_font_smoothing_off((font, font_size): OtherProps) -> impl Scene {
    bsn! [
        Text2d::new("This text has\nFontSmoothing::None\nAnd Justify::Center")
        TextFont {
            font: FontSourceTemplate::Handle(font),
            font_size: font_size,
            font_smoothing: FontSmoothing::None,
        }
        TextLayout::new_with_justify(Justify::Center)
        Transform::from_translation(Vec3::new(-400.0, -250.0, 0.0))
        // Add a black shadow to the text
        Text2dShadow::default()
    ]
}

fn demonstrate_anchors(other_props: OtherProps) -> impl Scene {
    fn make_child(
        (font, font_size): OtherProps,
        (text_anchor, color): (Anchor, Color),
    ) -> impl Scene {
        bsn![
            Text2d::new(" Anchor".to_string())
            TextFont {
                font: FontSourceTemplate::Handle(font),
                font_size: font_size,
            }
            // TODO: what can be done here instead?
            Anchor({text_anchor.as_vec()})
            TextBackgroundColor({Color::WHITE.darker(0.8)})
            Transform::from_translation(-1. * Vec3::Z)
            Children[
                (
                    TextSpan({"::".to_string()})
                    TextFont {
                        font: FontSourceTemplate::Handle(font),
                        font_size: font_size,
                    }
                    TextColor(Color::from(LIGHT_GREY))
                    TextBackgroundColor(Color::from(DARK_BLUE))
                ),
                (
                    TextSpan({format!("{text_anchor:?} ")})
                    TextFont {
                        font: FontSourceTemplate::Handle(font),
                        font_size: font_size,
                    }
                    TextColor(color)
                    TextBackgroundColor(Color::from(color.darker(0.3)))
                )
            ]
        ]
    }

    bsn![
        Sprite {
            color: Color::Srgba(LIGHT_CYAN),
            custom_size: Vec2::new(10., 10.),
        }
        Transform::from_translation(250. * Vec3::Y)
        Children[
            make_child(other_props, (Anchor::TOP_LEFT, Color::Srgba(LIGHT_SALMON))),
            make_child(other_props, (Anchor::TOP_RIGHT, Color::Srgba(LIGHT_GREEN))),
            make_child(other_props, (Anchor::BOTTOM_RIGHT, Color::Srgba(LIGHT_BLUE))),
            make_child(other_props, (Anchor::BOTTOM_LEFT, Color::Srgba(LIGHT_YELLOW))),
        ]
    ]
}

fn animate_translation(
    time: Res<Time>,
    mut query: Query<&mut Transform, (With<Text2d>, With<AnimateTranslation>)>,
) {
    for mut transform in &mut query {
        transform.translation.x = 100.0 * ops::sin(time.elapsed_secs()) - 400.0;
        transform.translation.y = 100.0 * ops::cos(time.elapsed_secs());
    }
}

fn animate_rotation(
    time: Res<Time>,
    mut query: Query<&mut Transform, (With<Text2d>, With<AnimateRotation>)>,
) {
    for mut transform in &mut query {
        transform.rotation = Quat::from_rotation_z(ops::cos(time.elapsed_secs()));
    }
}

fn animate_scale(
    time: Res<Time>,
    mut query: Query<&mut Transform, (With<Text2d>, With<AnimateScale>)>,
) {
    // Consider changing font-size instead of scaling the transform. Scaling a Text2D will scale the
    // rendered quad, resulting in a pixellated look.
    for mut transform in &mut query {
        let scale = (ops::sin(time.elapsed_secs()) + 1.1) * 2.0;
        transform.scale.x = scale;
        transform.scale.y = scale;
    }
}
