//! Stress test scaffolding for chunk-oriented instance benchmarks.
//!
//! Run with:
//! `cargo run --example many_chunk_instances --release -- --help`

mod benchmark;

use std::str::FromStr;

use argh::FromArgs;
use bevy::{
    diagnostic::{
        FrameCount, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};

#[derive(FromArgs, Resource)]
/// `many_chunk_instances` stress test
struct Args {
    /// the benchmark mode: `static`, `sparse`, or `mixed`
    #[argh(option, default = "Mode::Static")]
    mode: Mode,

    /// the number of visible/static instances used in `static` mode
    #[argh(option, default = "1_000_000")]
    instance_count: usize,

    /// the number of simulation entities used in `sparse` and `mixed` modes
    #[argh(option, default = "10_000_000")]
    simulation_count: usize,

    /// the fraction of entities near the camera in sparse mode
    #[argh(option, default = "0.02")]
    near_fraction: f32,

    /// the fraction of entities updated each frame in mixed mode
    #[argh(option, default = "0.01")]
    dirty_fraction: f32,

    /// whether to step the camera by a fixed amount each frame
    #[argh(switch)]
    benchmark: bool,

    /// enable the experimental main-world mass-instance chunk index
    #[argh(switch)]
    experimental_mass_chunks: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Mode {
    #[default]
    Static,
    Sparse,
    Mixed,
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "static" => Ok(Self::Static),
            "sparse" => Ok(Self::Sparse),
            "mixed" => Ok(Self::Mixed),
            _ => Err(format!(
                "Unknown mode '{value}', valid options: 'static', 'sparse', 'mixed'"
            )),
        }
    }
}

#[derive(Component)]
struct MixedDirtyEntity(u32);

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
        SystemInformationDiagnosticsPlugin,
        bevy::render::diagnostic::RenderBenchmarkDiagnosticsPlugin,
        bevy::render::diagnostic::MeshAllocatorDiagnosticPlugin,
        bevy::pbr::diagnostic::PbrBenchmarkDiagnosticsPlugin,
        bevy::pbr::diagnostic::MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::default(),
        benchmark::BenchmarkOutputPlugin::new("many_chunk_instances"),
    ))
    .insert_resource(benchmark_metadata(&args))
    .insert_resource(WinitSettings::continuous())
    .insert_resource(args)
    .add_systems(Startup, setup)
    .add_systems(Update, orbit_camera);

    if app.world().resource::<Args>().experimental_mass_chunks {
        app.add_plugins(bevy::pbr::experimental::MassInstanceRenderingPlugin);
    }

    app.add_systems(
        Update,
        move_dirty_entities.run_if(resource_equals(Mode::Mixed)),
    );

    app.run();
}

fn benchmark_metadata(args: &Args) -> benchmark::BenchmarkMetadata {
    benchmark::BenchmarkMetadata(
        [
            ("mode".into(), mode_name(args.mode).into()),
            ("instance_count".into(), args.instance_count.to_string()),
            ("simulation_count".into(), args.simulation_count.to_string()),
            ("near_fraction".into(), args.near_fraction.to_string()),
            ("dirty_fraction".into(), args.dirty_fraction.to_string()),
            ("benchmark".into(), args.benchmark.to_string()),
            (
                "experimental_mass_chunks".into(),
                args.experimental_mass_chunks.to_string(),
            ),
        ]
        .into_iter()
        .collect(),
    )
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Cuboid::from_size(Vec3::splat(1.0)));
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 250.0, 450.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    match args.mode {
        Mode::Static => spawn_static_instances(
            &mut commands,
            mesh.clone(),
            material.clone(),
            args.instance_count,
        ),
        Mode::Sparse => spawn_sparse_visibility_instances(
            &mut commands,
            mesh.clone(),
            material.clone(),
            args.simulation_count,
            args.near_fraction,
        ),
        Mode::Mixed => spawn_mixed_dirty_instances(
            &mut commands,
            mesh.clone(),
            material,
            args.simulation_count,
        ),
    }
}

fn spawn_static_instances(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    count: usize,
) {
    let side = cube_side(count);
    commands.spawn_batch((0..count).map(move |index| {
        (
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(grid_position(index, side, 2.5)),
        )
    }));
}

fn spawn_sparse_visibility_instances(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    count: usize,
    near_fraction: f32,
) {
    let near_count = ((count as f32) * near_fraction.clamp(0.0, 1.0)).round() as usize;
    let near_side = cube_side(near_count);
    commands.spawn_batch((0..count).map(move |index| {
        let translation = if index < near_count {
            grid_position(index, near_side, 2.5)
        } else {
            far_field_position(index - near_count)
        };

        (
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(translation),
        )
    }));
}

fn spawn_mixed_dirty_instances(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    count: usize,
) {
    let side = cube_side(count);
    commands.spawn_batch((0..count).map(move |index| {
        (
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(grid_position(index, side, 2.5)),
            MixedDirtyEntity(index as u32),
        )
    }));
}

fn move_dirty_entities(
    args: Res<Args>,
    frame_count: Res<FrameCount>,
    mut entities: Query<(&MixedDirtyEntity, &mut Transform)>,
) {
    let stride = ((1.0 / args.dirty_fraction.max(f32::EPSILON)).round() as u32).max(1);
    let frame_mod = frame_count.0 % stride;

    for (entity, mut transform) in &mut entities {
        if entity.0 % stride != frame_mod {
            continue;
        }

        let phase = frame_count.0 as f32 * 0.01 + entity.0 as f32 * 0.0001;
        transform.translation.y = phase.sin() * 6.0;
        transform.rotate_y(0.01);
    }
}

fn orbit_camera(
    args: Res<Args>,
    frame_count: Res<FrameCount>,
    mut cameras: Query<&mut Transform, With<Camera>>,
) {
    let angle = if args.benchmark {
        frame_count.0 as f32 * 0.003
    } else {
        frame_count.0 as f32 * 0.0015
    };
    let radius = 450.0;
    let position = Vec3::new(angle.cos() * radius, 250.0, angle.sin() * radius);

    for mut transform in &mut cameras {
        *transform = Transform::from_translation(position).looking_at(Vec3::ZERO, Vec3::Y);
    }
}

fn cube_side(count: usize) -> usize {
    (count.max(1) as f32).cbrt().ceil() as usize
}

fn grid_position(index: usize, side: usize, spacing: f32) -> Vec3 {
    let x = index % side;
    let y = (index / side) % side;
    let z = index / (side * side);
    let half = side as f32 * spacing * 0.5;

    Vec3::new(
        x as f32 * spacing - half,
        y as f32 * spacing - half,
        z as f32 * spacing - half,
    )
}

fn far_field_position(index: usize) -> Vec3 {
    let layer = index / 4096;
    let offset = (index % 4096) as f32 * 3.0;
    Vec3::new(250_000.0 + offset, (layer % 256) as f32 * 3.0, 250_000.0)
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Static => "static",
        Mode::Sparse => "sparse",
        Mode::Mixed => "mixed",
    }
}

fn resource_equals(expected: Mode) -> impl Fn(Res<Args>) -> bool + Clone {
    move |args: Res<Args>| args.mode == expected
}
