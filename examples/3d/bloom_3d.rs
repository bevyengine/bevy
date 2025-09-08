//! Illustrates bloom post-processing using HDR and emissive materials.

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    math::ops,
    post_process::bloom::{Bloom, BloomCompositeMode},
    prelude::*,
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (update_bloom_settings, bounce_spheres))
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Tonemapping::TonyMcMapface, // 1. Using a tonemapper that desaturates to white is recommended
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Bloom::NATURAL, // 2. Enable bloom for the camera
    ));

    let material_emissive1 = materials.add(StandardMaterial {
        emissive: LinearRgba::rgb(0.0, 0.0, 150.0), // 3. Put something bright in a dark environment to see the effect
        ..default()
    });
    let material_emissive2 = materials.add(StandardMaterial {
        emissive: LinearRgba::rgb(1000.0, 1000.0, 1000.0),
        ..default()
    });
    let material_emissive3 = materials.add(StandardMaterial {
        emissive: LinearRgba::rgb(50.0, 0.0, 0.0),
        ..default()
    });
    let material_non_emissive = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        ..default()
    });

    let mesh = meshes.add(Sphere::new(0.4).mesh().ico(5).unwrap());

    for x in -5..5 {
        for z in -5..5 {
            // This generates a pseudo-random integer between `[0, 6)`, but deterministically so
            // the same spheres are always the same colors.
            let mut hasher = DefaultHasher::new();
            (x, z).hash(&mut hasher);
            let rand = (hasher.finish() + 3) % 6;

            let (material, scale) = match rand {
                0 => (material_emissive1.clone(), 0.5),
                1 => (material_emissive2.clone(), 0.1),
                2 => (material_emissive3.clone(), 1.0),
                3..=5 => (material_non_emissive.clone(), 1.5),
                _ => unreachable!(),
            };

            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(x as f32 * 2.0, 0.0, z as f32 * 2.0)
                    .with_scale(Vec3::splat(scale)),
                Bouncing,
            ));
        }
    }

    // example instructions
    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
    ));
}

// ------------------------------------------------------------------------------------------------

fn update_bloom_settings(
    camera: Single<(Entity, Option<&mut Bloom>), With<Camera>>,
    mut text: Single<&mut Text>,
    mut commands: Commands,
    keycode: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let bloom = camera.into_inner();

    match bloom {
        (entity, Some(mut bloom)) => {
            text.0 = "Bloom (Toggle: Space)\n".to_string();
            text.push_str(&format!("(Q/A) Intensity: {:.2}\n", bloom.intensity));
            text.push_str(&format!(
                "(W/S) Low-frequency boost: {:.2}\n",
                bloom.low_frequency_boost
            ));
            text.push_str(&format!(
                "(E/D) Low-frequency boost curvature: {:.2}\n",
                bloom.low_frequency_boost_curvature
            ));
            text.push_str(&format!(
                "(R/F) High-pass frequency: {:.2}\n",
                bloom.high_pass_frequency
            ));
            text.push_str(&format!(
                "(T/G) Mode: {}\n",
                match bloom.composite_mode {
                    BloomCompositeMode::EnergyConserving => "Energy-conserving",
                    BloomCompositeMode::Additive => "Additive",
                }
            ));
            text.push_str(&format!(
                "(Y/H) Threshold: {:.2}\n",
                bloom.prefilter.threshold
            ));
            text.push_str(&format!(
                "(U/J) Threshold softness: {:.2}\n",
                bloom.prefilter.threshold_softness
            ));
            text.push_str(&format!("(I/K) Horizontal Scale: {:.2}\n", bloom.scale.x));

            if keycode.just_pressed(KeyCode::Space) {
                commands.entity(entity).remove::<Bloom>();
            }

            let dt = time.delta_secs();

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

            if keycode.pressed(KeyCode::KeyK) {
                bloom.scale.x -= dt * 2.0;
            }
            if keycode.pressed(KeyCode::KeyI) {
                bloom.scale.x += dt * 2.0;
            }
            bloom.scale.x = bloom.scale.x.clamp(0.0, 8.0);
        }

        (entity, None) => {
            text.0 = "Bloom: Off (Toggle: Space)".to_string();

            if keycode.just_pressed(KeyCode::Space) {
                commands.entity(entity).insert(Bloom::NATURAL);
            }
        }
    }
}

#[derive(Component)]
struct Bouncing;

fn bounce_spheres(time: Res<Time>, mut query: Query<&mut Transform, With<Bouncing>>) {
    for mut transform in query.iter_mut() {
        transform.translation.y =
            ops::sin(transform.translation.x + transform.translation.z + time.elapsed_secs());
    }
}
