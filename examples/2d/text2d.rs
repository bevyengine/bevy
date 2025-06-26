//! Shows text rendering with moving, rotating and scaling text.
//!
//! Note that this uses [`Text2d`] to display text alongside your other entities in a 2D scene.
//!
//! For an example on how to render text as part of a user interface, independent from the world
//! viewport, you may want to look at `games/contributors.rs` or `ui/text.rs`.

use bevy::{
    color::palettes::css::*,
    math::ops,
    prelude::*,
    sprite::Anchor,
    text::{FontSmoothing, LineBreak, TextBounds},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (animate_translation, animate_rotation, animate_scale),
        )
        .run();
}

#[derive(Component)]
struct AnimateTranslation;

#[derive(Component)]
struct AnimateRotation;

#[derive(Component)]
struct AnimateScale;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_font = TextFont {
        font: font.clone(),
        font_size: 50.0,
        ..default()
    };
    let text_justification = Justify::Center;
    commands.spawn(Camera2d);
    // Demonstrate changing translation
    commands.spawn((
        Text2d::new("translation"),
        text_font.clone(),
        TextLayout::new_with_justify(text_justification),
        AnimateTranslation,
    ));
    // Demonstrate changing rotation
    commands.spawn((
        Text2d::new("rotation"),
        text_font.clone(),
        TextLayout::new_with_justify(text_justification),
        AnimateRotation,
    ));
    // Demonstrate changing scale
    commands.spawn((
        Text2d::new("scale"),
        text_font,
        TextLayout::new_with_justify(text_justification),
        Transform::from_translation(Vec3::new(400.0, 0.0, 0.0)),
        AnimateScale,
    ));
    // Demonstrate text wrapping
    let slightly_smaller_text_font = TextFont {
        font,
        font_size: 35.0,
        ..default()
    };
    let box_size = Vec2::new(300.0, 200.0);
    let box_position = Vec2::new(0.0, -250.0);
    commands.spawn((
        Sprite::from_color(Color::srgb(0.25, 0.25, 0.55), box_size),
        Transform::from_translation(box_position.extend(0.0)),
        children![(
            Text2d::new("this text wraps in the box\n(Unicode linebreaks)"),
            slightly_smaller_text_font.clone(),
            TextLayout::new(Justify::Left, LineBreak::WordBoundary),
            // Wrap text in the rectangle
            TextBounds::from(box_size),
            // Ensure the text is drawn on top of the box
            Transform::from_translation(Vec3::Z),
        )],
    ));

    let other_box_size = Vec2::new(300.0, 200.0);
    let other_box_position = Vec2::new(320.0, -250.0);
    commands.spawn((
        Sprite::from_color(Color::srgb(0.25, 0.25, 0.55), other_box_size),
        Transform::from_translation(other_box_position.extend(0.0)),
        children![(
            Text2d::new("this text wraps in the box\n(AnyCharacter linebreaks)"),
            slightly_smaller_text_font.clone(),
            TextLayout::new(Justify::Left, LineBreak::AnyCharacter),
            // Wrap text in the rectangle
            TextBounds::from(other_box_size),
            // Ensure the text is drawn on top of the box
            Transform::from_translation(Vec3::Z),
        )],
    ));

    // Demonstrate font smoothing off
    commands.spawn((
        Text2d::new("This text has\nFontSmoothing::None\nAnd Justify::Center"),
        slightly_smaller_text_font
            .clone()
            .with_font_smoothing(FontSmoothing::None),
        TextLayout::new_with_justify(Justify::Center),
        Transform::from_translation(Vec3::new(-400.0, -250.0, 0.0)),
    ));

    commands
        .spawn((
            Sprite {
                color: Color::Srgba(LIGHT_CYAN),
                custom_size: Some(Vec2::new(10., 10.)),
                ..Default::default()
            },
            Transform::from_translation(250. * Vec3::Y),
        ))
        .with_children(|commands| {
            for (text_anchor, color) in [
                (Anchor::TOP_LEFT, Color::Srgba(LIGHT_SALMON)),
                (Anchor::TOP_RIGHT, Color::Srgba(LIGHT_GREEN)),
                (Anchor::BOTTOM_RIGHT, Color::Srgba(LIGHT_BLUE)),
                (Anchor::BOTTOM_LEFT, Color::Srgba(LIGHT_YELLOW)),
            ] {
                commands
                    .spawn((
                        Text2d::new(" Anchor".to_string()),
                        slightly_smaller_text_font.clone(),
                        text_anchor,
                    ))
                    .with_child((
                        TextSpan("::".to_string()),
                        slightly_smaller_text_font.clone(),
                        TextColor(LIGHT_GREY.into()),
                    ))
                    .with_child((
                        TextSpan(format!("{text_anchor:?} ")),
                        slightly_smaller_text_font.clone(),
                        TextColor(color),
                    ));
            }
        });
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
