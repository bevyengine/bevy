//! Illustrates bloom configuration using HDR and emissive materials.

use bevy::{core_pipeline::bloom::BloomSettings, prelude::*};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_scene)
        .add_system(update_bloom_settings)
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
                hdr: true, // 1. HDR must be enabled on the camera
                ..default()
            },
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        BloomSettings::default(), // 2. Enable bloom for the camera
    ));

    let material_emissive = materials.add(StandardMaterial {
        emissive: Color::rgb_linear(5.2, 1.2, 0.8), // 3. Set StandardMaterial::emissive using Color::rgb_linear, for entities we want to apply bloom to
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
            let rand = hasher.finish() % 2 == 0;

            let material = if rand {
                material_emissive.clone()
            } else {
                material_non_emissive.clone()
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
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            ..default()
        }),
    );
}

// ------------------------------------------------------------------------------------------------

fn update_bloom_settings(
    mut camera: Query<&mut BloomSettings>,
    mut text: Query<&mut Text>,
    keycode: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let mut bloom_settings = camera.single_mut();
    let mut text = text.single_mut();
    let text = &mut text.sections[0].value;

    *text = "BloomSettings\n".to_string();
    text.push_str("-------------\n");
    text.push_str(&format!("Threshold: {}\n", bloom_settings.threshold));
    text.push_str(&format!("Knee: {}\n", bloom_settings.knee));
    text.push_str(&format!("Scale: {}\n", bloom_settings.scale));
    text.push_str(&format!("Intensity: {}\n", bloom_settings.intensity));

    text.push_str("\n\n");

    text.push_str("Controls (-/+)\n");
    text.push_str("---------------\n");
    text.push_str("Q/W - Threshold\n");
    text.push_str("E/R - Knee\n");
    text.push_str("A/S - Scale\n");
    text.push_str("D/F - Intensity\n");

    let dt = time.delta_seconds();

    if keycode.pressed(KeyCode::Q) {
        bloom_settings.threshold -= dt;
    }
    if keycode.pressed(KeyCode::W) {
        bloom_settings.threshold += dt;
    }

    if keycode.pressed(KeyCode::E) {
        bloom_settings.knee -= dt;
    }
    if keycode.pressed(KeyCode::R) {
        bloom_settings.knee += dt;
    }

    if keycode.pressed(KeyCode::A) {
        bloom_settings.scale -= dt;
    }
    if keycode.pressed(KeyCode::S) {
        bloom_settings.scale += dt;
    }

    if keycode.pressed(KeyCode::D) {
        bloom_settings.intensity -= dt;
    }
    if keycode.pressed(KeyCode::F) {
        bloom_settings.intensity += dt;
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
