//! Benchmark to test rendering many animated materials
use argh::FromArgs;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin, WindowResolution},
};
use std::f32::consts::PI;

#[derive(FromArgs, Resource)]
/// Command-line arguments for the `many_materials` stress test.
struct Args {
    /// the size of the grid of materials to render (n x n)
    #[argh(option, short = 'n', default = "10")]
    grid_size: usize,
}

fn main() {
    // `from_env` panics on the web
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

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
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .insert_resource(args)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_materials)
        .run();
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    mesh_assets: ResMut<Assets<Mesh>>,
    material_assets: ResMut<Assets<StandardMaterial>>,
) {
    let args = args.into_inner();
    let material_assets = material_assets.into_inner();
    let mesh_assets = mesh_assets.into_inner();
    let n = args.grid_size;

    // Camera
    let w = n as f32;
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(w * 1.25, w + 1.0, w * 1.25)
            .looking_at(Vec3::new(0.0, (w * -1.1) + 1.0, 0.0), Vec3::Y),
    ));

    // Light
    commands.spawn((
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        DirectionalLight {
            illuminance: 3000.0,
            shadows_enabled: true,
            ..default()
        },
    ));

    // Cubes
    let mesh_handle = mesh_assets.add(Cuboid::from_size(Vec3::ONE));
    for x in 0..n {
        for z in 0..n {
            commands.spawn((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(material_assets.add(Color::WHITE)),
                Transform::from_translation(Vec3::new(x as f32, 0.0, z as f32)),
            ));
        }
    }
}

fn animate_materials(
    material_handles: Query<&MeshMaterial3d<StandardMaterial>>,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (i, material_handle) in material_handles.iter().enumerate() {
        if let Some(material) = materials.get_mut(material_handle) {
            let color = Color::hsl(
                ((i as f32 * 2.345 + time.elapsed_secs()) * 100.0) % 360.0,
                1.0,
                0.5,
            );
            material.base_color = color;
        }
    }
}
