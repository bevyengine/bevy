//! Demonstrates volumetric fog and lighting (light shafts or god rays).

use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping, Skybox},
    math::vec3,
    pbr::{VolumetricFogSettings, VolumetricLight},
    prelude::*,
};

const DIRECTIONAL_LIGHT_MOVEMENT_SPEED: f32 = 0.02;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::Srgba(Srgba {
            red: 0.02,
            green: 0.02,
            blue: 0.02,
            alpha: 1.0,
        })))
        .insert_resource(AmbientLight::NONE)
        .add_systems(Startup, setup)
        .add_systems(Update, tweak_scene)
        .add_systems(Update, move_directional_light)
        .run();
}

/// Initializes the scene.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn the glTF scene.
    commands.spawn(SceneBundle {
        scene: asset_server.load(
            GltfAssetLabel::Scene(0)
                .from_asset("models/VolumetricFogExample/VolumetricFogExample.glb"),
        ),
        ..default()
    });

    // Spawn the camera. Add the volumetric fog.
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(-1.7, 1.5, 4.5)
                .looking_at(vec3(-1.5, 1.7, 3.5), Vec3::Y),
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        })
        .insert(Tonemapping::TonyMcMapface)
        .insert(BloomSettings::default())
        .insert(Skybox {
            image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            brightness: 1000.0,
        })
        .insert(VolumetricFogSettings {
            // This value is explicitly set to 0 since we have no environment map light
            ambient_intensity: 0.0,
            ..default()
        });

    // Add the help text.
    commands.spawn(
        TextBundle {
            text: Text::from_section(
                "Press WASD or the arrow keys to change the light direction",
                TextStyle::default(),
            ),
            ..default()
        }
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

/// A system that makes directional lights in the glTF scene into volumetric
/// lights with shadows.
fn tweak_scene(
    mut commands: Commands,
    mut lights: Query<(Entity, &mut DirectionalLight), Changed<DirectionalLight>>,
) {
    for (light, mut directional_light) in lights.iter_mut() {
        // Shadows are needed for volumetric lights to work.
        directional_light.shadows_enabled = true;
        commands.entity(light).insert(VolumetricLight);
    }
}

/// Processes user requests to move the directional light.
fn move_directional_light(
    input: Res<ButtonInput<KeyCode>>,
    mut directional_lights: Query<&mut Transform, With<DirectionalLight>>,
) {
    let mut delta_theta = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp) {
        delta_theta.y += DIRECTIONAL_LIGHT_MOVEMENT_SPEED;
    }
    if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown) {
        delta_theta.y -= DIRECTIONAL_LIGHT_MOVEMENT_SPEED;
    }
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft) {
        delta_theta.x += DIRECTIONAL_LIGHT_MOVEMENT_SPEED;
    }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight) {
        delta_theta.x -= DIRECTIONAL_LIGHT_MOVEMENT_SPEED;
    }

    if delta_theta == Vec2::ZERO {
        return;
    }

    let delta_quat = Quat::from_euler(EulerRot::XZY, delta_theta.y, 0.0, delta_theta.x);
    for mut transform in directional_lights.iter_mut() {
        transform.rotate(delta_quat);
    }
}
