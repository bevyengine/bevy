//! Shows text rendering with moving, rotating and scaling text.
//!
//! Note that this uses [`Text2dBundle`] to display text alongside your other entities in a 2D scene.
//!
//! For an example on how to render text as part of a user interface, independent from the world
//! viewport, you may want to look at `2d/contributors.rs` or `ui/text.rs`.

use bevy::{prelude::*, text::Text2dBounds};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(animate_translation)
        .add_system(animate_rotation)
        .add_system(animate_scale)
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
        font,
        font_size: 60.0,
        color: Color::WHITE,
    };
    let text_alignment = TextAlignment::CENTER;
    // 2d camera
    commands.spawn_bundle(Camera2dBundle::default());
    // Demonstrate changing translation
    commands
        .spawn_bundle(Text2dBundle {
            text: Text::from_section("translation", text_style.clone())
                .with_alignment(text_alignment),
            ..default()
        })
        .insert(AnimateTranslation);
    // Demonstrate changing rotation
    commands
        .spawn_bundle(Text2dBundle {
            text: Text::from_section("rotation", text_style.clone()).with_alignment(text_alignment),
            ..default()
        })
        .insert(AnimateRotation);
    // Demonstrate changing scale
    commands
        .spawn_bundle(Text2dBundle {
            text: Text::from_section("scale", text_style.clone()).with_alignment(text_alignment),
            ..default()
        })
        .insert(AnimateScale);
    // Demonstrate text wrapping
    let box_size = Vec2::new(300.0, 200.0);
    let box_position = Vec2::new(0.0, -250.0);
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0.25, 0.25, 0.75),
            custom_size: Some(Vec2::new(box_size.x, box_size.y)),
            ..default()
        },
        transform: Transform::from_translation(box_position.extend(0.0)),
        ..default()
    });
    commands.spawn_bundle(Text2dBundle {
        text: Text::from_section("this text wraps in the box", text_style),
        text_2d_bounds: Text2dBounds {
            // Wrap text in the rectangle
            size: box_size,
        },
        // We align text to the top-left, so this transform is the top-left corner of our text. The
        // box is centered at box_position, so it is necessary to move by half of the box size to
        // keep the text in the box.
        transform: Transform::from_xyz(
            box_position.x - box_size.x / 2.0,
            box_position.y + box_size.y / 2.0,
            1.0,
        ),
        ..default()
    });
}

fn animate_translation(
    time: Res<Time>,
    mut query: Query<&mut Transform, (With<Text>, With<AnimateTranslation>)>,
) {
    for mut transform in &mut query {
        transform.translation.x = 100.0 * time.seconds_since_startup().sin() as f32 - 400.0;
        transform.translation.y = 100.0 * time.seconds_since_startup().cos() as f32;
    }
}

fn animate_rotation(
    time: Res<Time>,
    mut query: Query<&mut Transform, (With<Text>, With<AnimateRotation>)>,
) {
    for mut transform in &mut query {
        transform.rotation = Quat::from_rotation_z(time.seconds_since_startup().cos() as f32);
    }
}

fn animate_scale(
    time: Res<Time>,
    mut query: Query<&mut Transform, (With<Text>, With<AnimateScale>)>,
) {
    // Consider changing font-size instead of scaling the transform. Scaling a Text2D will scale the
    // rendered quad, resulting in a pixellated look.
    for mut transform in &mut query {
        transform.translation = Vec3::new(400.0, 0.0, 0.0);
        transform.scale = Vec3::splat((time.seconds_since_startup().sin() as f32 + 1.1) * 2.0);
    }
}
