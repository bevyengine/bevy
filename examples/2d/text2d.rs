//! Shows text rendering with moving, rotating and scaling text.
//!
//! Note that this uses [`Text2dBundle`] to display text alongside your other entities in a 2D scene.
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
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 50.0,
        ..default()
    };
    let text_justification = JustifyText::Center;
    // 2d camera
    commands.spawn(Camera2dBundle::default());
    // Demonstrate changing translation
    commands.spawn((
        Text2d::new("translation"),
        text_style.clone(),
        TextBlock::new_with_justify(text_justification),
        AnimateTranslation,
    ));
    // Demonstrate changing rotation
    commands.spawn((
        Text2d::new("rotation"),
        text_style.clone(),
        TextBlock::new_with_justify(text_justification),
        AnimateRotation,
    ));
    // Demonstrate changing scale
    commands.spawn((
        Text2d::new("scale"),
        text_style,
        TextBlock::new_with_justify(text_justification),
        Transform::from_translation(Vec3::new(400.0, 0.0, 0.0)),
        AnimateScale,
    ));
    // Demonstrate text wrapping
    let slightly_smaller_text_style = TextStyle {
        font,
        font_size: 35.0,
        ..default()
    };
    let box_size = Vec2::new(300.0, 200.0);
    let box_position = Vec2::new(0.0, -250.0);
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(box_size.x, box_size.y)),
                ..default()
            },
            transform: Transform::from_translation(box_position.extend(0.0)),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn((
                Text2d::new("this text wraps in the box\n(Unicode linebreaks)"),
                slightly_smaller_text_style.clone(),
                TextBlock::new_with_justify(JustifyText::Left)
                    .with_linebreak(LineBreak::WordBoundary),
                // Wrap text in the rectangle
                TextBounds::from(box_size),
                // ensure the text is drawn on top of the box
                Transform::from_translation(Vec3::Z),
            ));
        });

    let other_box_size = Vec2::new(300.0, 200.0);
    let other_box_position = Vec2::new(320.0, -250.0);
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.20, 0.3, 0.70),
                custom_size: Some(Vec2::new(other_box_size.x, other_box_size.y)),
                ..default()
            },
            transform: Transform::from_translation(other_box_position.extend(0.0)),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn((
                Text2d::new("this text wraps in the box\n(AnyCharacter linebreaks)"),
                slightly_smaller_text_style.clone(),
                TextBlock::new_with_justify(JustifyText::Left)
                    .with_linebreak(LineBreak::AnyCharacter),
                // Wrap text in the rectangle
                TextBounds::from(other_box_size),
                // ensure the text is drawn on top of the box
                Transform::from_translation(Vec3::Z),
            ));
        });

    // Demonstrate font smoothing off
    commands.spawn((
        Text2d::new("FontSmoothing::None"),
        slightly_smaller_text_style.clone(),
        TextBlock::new_with_font_smoothing(FontSmoothing::None),
        Transform::from_translation(Vec3::new(-400.0, -250.0, 0.0)),
    ));

    for (text_anchor, color) in [
        (Anchor::TopLeft, Color::Srgba(RED)),
        (Anchor::TopRight, Color::Srgba(LIME)),
        (Anchor::BottomRight, Color::Srgba(BLUE)),
        (Anchor::BottomLeft, Color::Srgba(YELLOW)),
    ] {
        commands.spawn((
            Text2d::new(format!(" Anchor::{text_anchor:?} ")),
            TextStyle {
                color,
                ..slightly_smaller_text_style.clone()
            },
            Transform::from_translation(250. * Vec3::Y),
            text_anchor,
        ));
    }
}

fn animate_translation(
    time: Res<Time>,
    mut query: Query<&mut Transform, (With<Text2d>, With<AnimateTranslation>)>,
) {
    for mut transform in &mut query {
        transform.translation.x = 100.0 * ops::sin(time.elapsed_seconds()) - 400.0;
        transform.translation.y = 100.0 * ops::cos(time.elapsed_seconds());
    }
}

fn animate_rotation(
    time: Res<Time>,
    mut query: Query<&mut Transform, (With<Text2d>, With<AnimateRotation>)>,
) {
    for mut transform in &mut query {
        transform.rotation = Quat::from_rotation_z(ops::cos(time.elapsed_seconds()));
    }
}

fn animate_scale(
    time: Res<Time>,
    mut query: Query<&mut Transform, (With<Text2d>, With<AnimateScale>)>,
) {
    // Consider changing font-size instead of scaling the transform. Scaling a Text2D will scale the
    // rendered quad, resulting in a pixellated look.
    for mut transform in &mut query {
        let scale = (ops::sin(time.elapsed_seconds()) + 1.1) * 2.0;
        transform.scale.x = scale;
        transform.scale.y = scale;
    }
}
