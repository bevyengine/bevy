//! A stress test for the Meshlet pipeline specialization overhead.
//!
//! Run with `--unique-materials` to trigger the unconditional specialization bug.
//! Run without it (shared material) to see the baseline performance.

use argh::FromArgs;
use bevy::{
    pbr::experimental::meshlet::{MeshletMesh3d, MeshletPlugin},
    prelude::*,
    winit::WinitSettings,
};

#[derive(FromArgs, Resource)]
#[argh(description = "Meshlet Material Stress Test")]
struct Args {
    /// the grid size (e.g., 50 means 50x50 = 2500 meshlets)
    #[argh(option, short = 'n', default = "50")]
    grid_size: usize,

    /// if set, every meshlet gets a unique material asset.
    /// This triggers the unconditional pipeline specialization bug in `prepare_material_meshlet_meshes`.
    #[argh(switch)]
    unique_materials: bool,
}

const ASSET_URL: &str =
    "https://github.com/bevyengine/bevy_asset_files/raw/6dccaef517bde74d1969734703709aead7211dbc/meshlet/bunny.meshlet_mesh";

fn main() {
    let args: Args = argh::from_env();

    println!("Meshlet Stress Test");
    println!(
        "Grid size: {}x{} ({} instances)",
        args.grid_size,
        args.grid_size,
        args.grid_size * args.grid_size
    );
    println!(
        "Materials: {}",
        if args.unique_materials {
            "UNIQUE"
        } else {
            "SHARED"
        }
    );

    App::new()
        .add_plugins((
            DefaultPlugins,
            MeshletPlugin {
                cluster_buffer_slots: 8192,
            },
        ))
        .insert_resource(WinitSettings::continuous())
        .insert_resource(args)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let meshlet_handle = asset_server.load(ASSET_URL);

    let n = args.grid_size;
    let spacing = 2.0;
    let offset = (n as f32 * spacing) / 2.0;

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, offset, offset * 1.5).looking_at(Vec3::ZERO, Vec3::Y),
        Msaa::Off,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 3000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            1.0,
            -std::f32::consts::FRAC_PI_4,
        )),
    ));

    let shared_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        ..default()
    });

    for x in 0..n {
        for z in 0..n {
            let material = if args.unique_materials {
                materials.add(StandardMaterial {
                    base_color: Color::srgb(x as f32 / n as f32, 0.5, z as f32 / n as f32),
                    ..default()
                })
            } else {
                shared_material.clone()
            };

            commands.spawn((
                MeshletMesh3d(meshlet_handle.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(
                    x as f32 * spacing - offset,
                    0.0,
                    z as f32 * spacing - offset,
                ),
            ));
        }
    }
}
