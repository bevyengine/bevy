//! Illustrates bloom post-processing in 2d.

use bevy::{
    core_pipeline::{
        bloom::{Bloom, BloomCompositeMode},
        tonemapping::Tonemapping,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_bloom_settings)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                hdr: true, // 1. HDR is required for bloom
                ..default()
            },
            tonemapping: Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
            ..default()
        },
        Bloom::default(), // 3. Enable bloom for the camera
    ));

    // Sprite
    commands.spawn(SpriteBundle {
        texture: asset_server.load("branding/bevy_bird_dark.png"),
        sprite: Sprite {
            color: Color::srgb(5.0, 5.0, 5.0), // 4. Put something bright in a dark environment to see the effect
            custom_size: Some(Vec2::splat(160.0)),
            ..default()
        },
        ..default()
    });

    // Circle mesh
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(100.))),
        // 4. Put something bright in a dark environment to see the effect
        MeshMaterial2d(materials.add(Color::srgb(7.5, 0.0, 7.5))),
        Transform::from_translation(Vec3::new(-200., 0., 0.)),
    ));

    // Hexagon mesh
    commands.spawn((
        Mesh2d(meshes.add(RegularPolygon::new(100., 6))),
        // 4. Put something bright in a dark environment to see the effect
        MeshMaterial2d(materials.add(Color::srgb(6.25, 9.4, 9.1))),
        Transform::from_translation(Vec3::new(200., 0., 0.)),
    ));

    // UI
    commands.spawn(
        TextBundle::from_section("", TextStyle::default()).with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

// ------------------------------------------------------------------------------------------------

fn update_bloom_settings(
    mut camera: Query<(Entity, Option<&mut Bloom>), With<Camera>>,
    mut text: Query<&mut Text>,
    mut commands: Commands,
    keycode: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let bloom = camera.single_mut();
    let mut text = text.single_mut();
    let text = &mut text.sections[0].value;

    match bloom {
        (entity, Some(mut bloom)) => {
            *text = "Bloom (Toggle: Space)\n".to_string();
            text.push_str(&format!("(Q/A) Intensity: {}\n", bloom.intensity));
            text.push_str(&format!(
                "(W/S) Low-frequency boost: {}\n",
                bloom.low_frequency_boost
            ));
            text.push_str(&format!(
                "(E/D) Low-frequency boost curvature: {}\n",
                bloom.low_frequency_boost_curvature
            ));
            text.push_str(&format!(
                "(R/F) High-pass frequency: {}\n",
                bloom.high_pass_frequency
            ));
            text.push_str(&format!(
                "(T/G) Mode: {}\n",
                match bloom.composite_mode {
                    BloomCompositeMode::EnergyConserving => "Energy-conserving",
                    BloomCompositeMode::Additive => "Additive",
                }
            ));
            text.push_str(&format!("(Y/H) Threshold: {}\n", bloom.prefilter.threshold));
            text.push_str(&format!(
                "(U/J) Threshold softness: {}\n",
                bloom.prefilter.threshold_softness
            ));

            if keycode.just_pressed(KeyCode::Space) {
                commands.entity(entity).remove::<Bloom>();
            }

            let dt = time.delta_seconds();

            if keycode.pressed(KeyCode::KeyA) {
                bloom.intensity -= dt / 10.0;
            }
            if keycode.pressed(KeyCode::KeyQ) {
                bloom.intensity += dt / 10.0;
            }
            bloom.intensity = bloom.intensity.clamp(0.0, 1.0);

            if keycode.pressed(KeyCode::KeyS) {
                bloom.low_frequency_boost -= dt / 10.0;
            }
            if keycode.pressed(KeyCode::KeyW) {
                bloom.low_frequency_boost += dt / 10.0;
            }
            bloom.low_frequency_boost = bloom.low_frequency_boost.clamp(0.0, 1.0);

            if keycode.pressed(KeyCode::KeyD) {
                bloom.low_frequency_boost_curvature -= dt / 10.0;
            }
            if keycode.pressed(KeyCode::KeyE) {
                bloom.low_frequency_boost_curvature += dt / 10.0;
            }
            bloom.low_frequency_boost_curvature =
                bloom.low_frequency_boost_curvature.clamp(0.0, 1.0);

            if keycode.pressed(KeyCode::KeyF) {
                bloom.high_pass_frequency -= dt / 10.0;
            }
            if keycode.pressed(KeyCode::KeyR) {
                bloom.high_pass_frequency += dt / 10.0;
            }
            bloom.high_pass_frequency = bloom.high_pass_frequency.clamp(0.0, 1.0);

            if keycode.pressed(KeyCode::KeyG) {
                bloom.composite_mode = BloomCompositeMode::Additive;
            }
            if keycode.pressed(KeyCode::KeyT) {
                bloom.composite_mode = BloomCompositeMode::EnergyConserving;
            }

            if keycode.pressed(KeyCode::KeyH) {
                bloom.prefilter.threshold -= dt;
            }
            if keycode.pressed(KeyCode::KeyY) {
                bloom.prefilter.threshold += dt;
            }
            bloom.prefilter.threshold = bloom.prefilter.threshold.max(0.0);

            if keycode.pressed(KeyCode::KeyJ) {
                bloom.prefilter.threshold_softness -= dt / 10.0;
            }
            if keycode.pressed(KeyCode::KeyU) {
                bloom.prefilter.threshold_softness += dt / 10.0;
            }
            bloom.prefilter.threshold_softness = bloom.prefilter.threshold_softness.clamp(0.0, 1.0);
        }

        (entity, None) => {
            *text = "Bloom: Off (Toggle: Space)".to_string();

            if keycode.just_pressed(KeyCode::Space) {
                commands.entity(entity).insert(Bloom::default());
            }
        }
    }
}
