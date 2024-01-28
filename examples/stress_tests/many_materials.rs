//! Benchmark to test rendering many animated materials

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    utils::HashSet,
    window::{PresentMode, WindowPlugin, WindowResolution},
};
use std::f32::consts::PI;

#[derive(Component)]
struct Floor;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1920.0, 1080.0)
                        .with_scale_factor_override(1.0),
                    title: "many_materials".into(),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (animate_materials, make_materials_unique))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let n = 4;

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(n as f32 + 1.0, 1.0, n as f32 + 1.0)
            .looking_at(Vec3::new(0.0, -0.5, 0.0), Vec3::Y),
        ..default()
    });

    // Plane
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Plane::from_size(50.0)),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3)),
            ..default()
        },
        Floor,
    ));

    // Light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        directional_light: DirectionalLight {
            illuminance: 3000.0,
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });

    // Helmets
    let helmet = asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0");
    for x in -n..=n {
        for z in -n..=n {
            commands.spawn(SceneBundle {
                scene: helmet.clone(),
                transform: Transform::from_translation(Vec3::new(x as f32, 0.0, z as f32)),
                ..default()
            });
        }
    }
}

fn animate_materials(
    material_handles: Query<&Handle<StandardMaterial>>,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (i, material_handle) in material_handles.iter().enumerate() {
        if let Some(material) = materials.get_mut(material_handle) {
            let color = Color::hsl(
                ((i as f32 * 2.345 + time.elapsed_seconds_wrapped()) * 100.0) % 360.0,
                1.0,
                0.5,
            );
            material.base_color = color;
        }
    }
}

/// This is needed because by default assets are loaded with shared materials
/// But we want to animate every helmet independently of the others, so we must duplicate the materials
fn make_materials_unique(
    mut material_handles: Query<&mut Handle<StandardMaterial>, Without<Floor>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ran: Local<bool>,
) {
    if *ran {
        return;
    }
    let mut set = HashSet::new();
    for mut material_handle in material_handles.iter_mut() {
        if set.contains(&material_handle.id()) {
            let material = materials.get(&*material_handle).unwrap().clone();
            *material_handle = materials.add(material);
        } else {
            set.insert(material_handle.id());
        }
        *ran = true;
    }
}
