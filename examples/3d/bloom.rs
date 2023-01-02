//! Illustrates bloom post-processing using HDR and emissive materials.

use bevy::{
    core_pipeline::bloom::{BloomCompositeMode, BloomSettings},
    prelude::*,
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_scene)
        .add_system(update_bloom_settings)
        .add_system(update_camera_settings)
        .add_system(bounce_spheres)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true, // 1. HDR can tremendously improve the look and realism of bloom
                ..default()
            },
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        BloomSettings::default(), // 2. Enable bloom for the camera
    ));

    let material_emissive1 = materials.add(StandardMaterial {
        emissive: Color::rgb_linear(500.0, 50.0, 5.0), // 3. Put something bright in a dark environment to see the effect
        ..default()
    });
    let material_emissive2 = materials.add(StandardMaterial {
        emissive: Color::rgb_linear(5.0, 500.0, 50.0),
        ..default()
    });
    let material_emissive3 = materials.add(StandardMaterial {
        emissive: Color::rgb_linear(50.0, 5.0, 500.0),
        ..default()
    });
    let material_non_emissive = materials.add(StandardMaterial {
        base_color: Color::GRAY,
        ..default()
    });

    let mesh = meshes.add(
        shape::Icosphere {
            radius: 0.5,
            subdivisions: 5,
        }
        .try_into()
        .unwrap(),
    );

    for x in -10..10 {
        for z in -10..10 {
            let mut hasher = DefaultHasher::new();
            (x, z).hash(&mut hasher);
            let rand = (hasher.finish() - 2) % 6;

            let material = match rand {
                0 => material_emissive1.clone(),
                1 => material_emissive2.clone(),
                2 => material_emissive3.clone(),
                3 | 4 | 5 => material_non_emissive.clone(),
                _ => unreachable!(),
            };

            commands.spawn((
                PbrBundle {
                    mesh: mesh.clone(),
                    material,
                    transform: Transform::from_xyz(x as f32 * 2.0, 0.0, z as f32 * 2.0),
                    ..default()
                },
                Bouncing,
            ));
        }
    }

    commands.spawn(
        TextBundle::from_section(
            "",
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 18.0,
                color: Color::BLACK,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            ..default()
        }),
    );
}

// ------------------------------------------------------------------------------------------------

fn update_bloom_settings(
    mut camera: Query<(Entity, Option<&mut BloomSettings>), With<Camera>>,
    mut text: Query<&mut Text>,
    mut commands: Commands,
    keycode: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let bloom_settings = camera.single_mut();
    let mut text = text.single_mut();
    let text = &mut text.sections[0].value;

    match bloom_settings {
        (entity, Some(mut bloom_settings)) => {
            *text = "BloomSettings (Toggle: Space)\n".to_string();
            text.push_str(&format!("(Q/A) Intensity: {}\n", bloom_settings.intensity));
            text.push_str(&format!("(W/S) LF boost: {}\n", bloom_settings.lf_boost));
            text.push_str(&format!(
                "(E/D) LF boost curv.: {}\n",
                bloom_settings.lf_boost_curvature
            ));
            text.push_str(&format!(
                "(R/F) High-pass freq.: {}\n",
                bloom_settings.high_pass_frequency
            ));
            text.push_str(&format!(
                "(T/G) Mode: {}\n",
                match bloom_settings.composite_mode {
                    BloomCompositeMode::EnergyConserving => "Energy-conserving",
                    BloomCompositeMode::Additive => "Additive",
                }
            ));
            text.push_str(&format!(
                "(Y/H) Threshold: {}\n",
                bloom_settings.prefilter_settings.threshold
            ));
            text.push_str(&format!(
                "(U/J) Threshld softness: {}\n",
                bloom_settings.prefilter_settings.threshold_softness
            ));

            if keycode.just_pressed(KeyCode::Space) {
                commands.entity(entity).remove::<BloomSettings>();
            }

            let dt = time.delta_seconds();

            if keycode.pressed(KeyCode::A) {
                bloom_settings.intensity -= dt / 10.0;
            }
            if keycode.pressed(KeyCode::Q) {
                bloom_settings.intensity += dt / 10.0;
            }
            bloom_settings.intensity = bloom_settings.intensity.clamp(0.0, 1.0);

            if keycode.pressed(KeyCode::S) {
                bloom_settings.lf_boost -= dt / 10.0;
            }
            if keycode.pressed(KeyCode::W) {
                bloom_settings.lf_boost += dt / 10.0;
            }
            bloom_settings.lf_boost = bloom_settings.lf_boost.clamp(0.0, 1.0);

            if keycode.pressed(KeyCode::D) {
                bloom_settings.lf_boost_curvature -= dt / 10.0;
            }
            if keycode.pressed(KeyCode::E) {
                bloom_settings.lf_boost_curvature += dt / 10.0;
            }
            bloom_settings.lf_boost_curvature = bloom_settings.lf_boost_curvature.clamp(0.0, 1.0);

            if keycode.pressed(KeyCode::F) {
                bloom_settings.high_pass_frequency -= dt / 10.0;
            }
            if keycode.pressed(KeyCode::R) {
                bloom_settings.high_pass_frequency += dt / 10.0;
            }
            bloom_settings.high_pass_frequency = bloom_settings.high_pass_frequency.clamp(0.0, 1.0);

            if keycode.pressed(KeyCode::G) {
                bloom_settings.composite_mode = BloomCompositeMode::Additive;
            }
            if keycode.pressed(KeyCode::T) {
                bloom_settings.composite_mode = BloomCompositeMode::EnergyConserving;
            }

            if keycode.pressed(KeyCode::H) {
                bloom_settings.prefilter_settings.threshold -= dt / 10.0;
            }
            if keycode.pressed(KeyCode::Y) {
                bloom_settings.prefilter_settings.threshold += dt / 10.0;
            }
            bloom_settings.prefilter_settings.threshold =
                bloom_settings.prefilter_settings.threshold.max(0.0);

            if keycode.pressed(KeyCode::J) {
                bloom_settings.prefilter_settings.threshold_softness -= dt / 10.0;
            }
            if keycode.pressed(KeyCode::U) {
                bloom_settings.prefilter_settings.threshold_softness += dt / 10.0;
            }
            bloom_settings.prefilter_settings.threshold_softness = bloom_settings
                .prefilter_settings
                .threshold_softness
                .clamp(0.0, 1.0);
        }

        (entity, None) => {
            *text = "Bloom: Off (Toggle: Space)\n".to_string();

            if keycode.just_pressed(KeyCode::Space) {
                commands.entity(entity).insert(BloomSettings::default());
            }
        }
    }
}

fn update_camera_settings(
    mut camera: Query<(Entity, &mut Camera), With<Camera>>,
    mut text: Query<&mut Text>,
    keycode: Res<Input<KeyCode>>,
) {
    let mut camera = camera.single_mut();
    let mut text = text.single_mut();
    let text = &mut text.sections[0].value;

    text.push_str("-----------------------------\n");
    text.push_str(&format!(
        "Camera HDR: {} (Toggle: Return)\n",
        match camera.1.hdr {
            true => "On",
            false => "Off",
        }
    ));

    if keycode.just_pressed(KeyCode::Return) {
        camera.1.hdr = !camera.1.hdr;
    }
}

#[derive(Component)]
struct Bouncing;

fn bounce_spheres(time: Res<Time>, mut query: Query<&mut Transform, With<Bouncing>>) {
    for mut transform in query.iter_mut() {
        transform.translation.y =
            (transform.translation.x + transform.translation.z + time.elapsed_seconds()).sin();
    }
}
