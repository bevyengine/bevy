//! Illustrates rectangular area lights and how surface roughness affects their appearance.

use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: bevy::window::PresentMode::Mailbox,
                    ..default()
                }),
                ..default()
            }),
            FreeCameraPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_gizmos, adjust_roughness))
        .run();
}

#[derive(Resource)]
struct FloorMaterial(Handle<StandardMaterial>);

#[derive(Component)]
struct RoughnessDisplay;

/// Simple scene with a sphere on a reflective floor, lit by two rectangular area lights
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let floor_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        metallic: 1.0,
        perceptual_roughness: 0.6,
        ..default()
    });
    commands.insert_resource(FloorMaterial(floor_material.clone()));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(floor_material),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));

    // Lights
    commands.spawn((
        RectLight {
            color: Color::srgb(1.0, 0.3, 0.2),
            intensity: 100_000.0,
            width: 2.0,
            height: 1.0,
            range: 20.0,
        },
        ShowLightGizmo::default(),
        Transform::from_xyz(1.0, 3.0, 1.0).looking_at(Vec3::Y, Vec3::Y),
    ));

    commands.spawn((
        RectLight {
            color: Color::srgb(0.5, 0.7, 1.0),
            intensity: 800_000.0,
            width: 1.5,
            height: 4.0,
            range: 20.0,
        },
        ShowLightGizmo::default(),
        Transform::from_xyz(-2.0, 1.5, -3.0)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-8.0, 5.0, 8.0).looking_at(Vec3::Y, Vec3::Y),
        FreeCamera::default(),
    ));

    commands.spawn((
        Text::new("Controls\nArrow Up/Down: Adjust floor roughness\nG: Toggle light gizmos\n\nRoughness: 0.60"),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        RoughnessDisplay,
    ));
}

/// Update floor roughness
fn adjust_roughness(
    keys: Res<ButtonInput<KeyCode>>,
    floor_material: Res<FloorMaterial>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut text_query: Query<&mut Text, With<RoughnessDisplay>>,
) {
    let delta = if keys.pressed(KeyCode::ArrowUp) {
        0.005
    } else if keys.pressed(KeyCode::ArrowDown) {
        -0.005
    } else {
        return;
    };

    if let Some(mut material) = materials.get_mut(&floor_material.0) {
        material.perceptual_roughness = (material.perceptual_roughness + delta).clamp(0.0, 1.0);

        if let Ok(mut text) = text_query.single_mut() {
            **text = format!(
                "Controls\nArrow Up/Down: Adjust floor roughness\nG: Toggle light gizmos\n\nRoughness: {:.2}",
                material.perceptual_roughness
            );
        }
    }
}

fn toggle_gizmos(keys: Res<ButtonInput<KeyCode>>, mut config_store: ResMut<GizmoConfigStore>) {
    if keys.just_pressed(KeyCode::KeyG) {
        let (config, light_config) = config_store.config_mut::<LightGizmoConfigGroup>();
        light_config.draw_all = false;
        config.enabled = !config.enabled;
    }
}
