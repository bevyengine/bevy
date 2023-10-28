//! Simple benchmark to test per-entity draw overhead.
//!
//! To measure performance realistically, be sure to run this in release mode.
//! `cargo run --example many_cubes --release`
//!
//! By default, this arranges the meshes in a spherical pattern that
//! distributes the meshes evenly.
//!
//! See `cargo run --example many_cubes --release -- --help` for more options.

use std::{f64::consts::PI, str::FromStr};

use argh::FromArgs;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::{DVec2, DVec3},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::{PresentMode, WindowPlugin, WindowResolution},
};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};

#[derive(FromArgs, Resource)]
/// `many_cubes` stress test
struct Args {
    /// how the cube instances should be positioned.
    #[argh(option, default = "Layout::Sphere")]
    layout: Layout,

    /// whether to step the camera animation by a fixed amount such that each frame is the same across runs.
    #[argh(switch)]
    benchmark: bool,

    /// whether to vary the material data in each instance.
    #[argh(switch)]
    vary_per_instance: bool,

    /// the number of different textures from which to randomly select the material base color. 0 means no textures.
    #[argh(option, default = "0")]
    material_texture_count: usize,
}

#[derive(Default, Clone)]
enum Layout {
    Cube,
    #[default]
    Sphere,
}

impl FromStr for Layout {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cube" => Ok(Self::Cube),
            "sphere" => Ok(Self::Sphere),
            _ => Err(format!(
                "Unknown layout value: '{}', valid options: 'cube', 'sphere'",
                s
            )),
        }
    }
}

fn main() {
    let args: Args = argh::from_env();

    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    resolution: WindowResolution::new(1920.0, 1080.0)
                        .with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
        ))
        .insert_resource(args)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_camera, print_mesh_count))
        .run();
}

const WIDTH: usize = 200;
const HEIGHT: usize = 200;

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    mut meshes: ResMut<Assets<Mesh>>,
    material_assets: ResMut<Assets<StandardMaterial>>,
    images: ResMut<Assets<Image>>,
) {
    warn!(include_str!("warning_string.txt"));

    let args = args.into_inner();
    let images = images.into_inner();
    let material_assets = material_assets.into_inner();

    let mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));

    let material_textures = init_textures(args, images);
    let materials = init_materials(args, &material_textures, material_assets);

    let mut material_rng = StdRng::seed_from_u64(42);
    match args.layout {
        Layout::Sphere => {
            // NOTE: This pattern is good for testing performance of culling as it provides roughly
            // the same number of visible meshes regardless of the viewing angle.
            const N_POINTS: usize = WIDTH * HEIGHT * 4;
            // NOTE: f64 is used to avoid precision issues that produce visual artifacts in the distribution
            let radius = WIDTH as f64 * 2.5;
            let golden_ratio = 0.5f64 * (1.0f64 + 5.0f64.sqrt());
            for i in 0..N_POINTS {
                let spherical_polar_theta_phi =
                    fibonacci_spiral_on_sphere(golden_ratio, i, N_POINTS);
                let unit_sphere_p = spherical_polar_to_cartesian(spherical_polar_theta_phi);
                commands.spawn(PbrBundle {
                    mesh: mesh.clone(),
                    material: materials.choose(&mut material_rng).unwrap().clone(),
                    transform: Transform::from_translation((radius * unit_sphere_p).as_vec3()),
                    ..default()
                });
            }

            // camera
            commands.spawn(Camera3dBundle::default());
        }
        _ => {
            // NOTE: This pattern is good for demonstrating that frustum culling is working correctly
            // as the number of visible meshes rises and falls depending on the viewing angle.
            for x in 0..WIDTH {
                for y in 0..HEIGHT {
                    // introduce spaces to break any kind of moiré pattern
                    if x % 10 == 0 || y % 10 == 0 {
                        continue;
                    }
                    // cube
                    commands.spawn(PbrBundle {
                        mesh: mesh.clone(),
                        material: materials.choose(&mut material_rng).unwrap().clone(),
                        transform: Transform::from_xyz((x as f32) * 2.5, (y as f32) * 2.5, 0.0),
                        ..default()
                    });
                    commands.spawn(PbrBundle {
                        mesh: mesh.clone(),
                        material: materials.choose(&mut material_rng).unwrap().clone(),
                        transform: Transform::from_xyz(
                            (x as f32) * 2.5,
                            HEIGHT as f32 * 2.5,
                            (y as f32) * 2.5,
                        ),
                        ..default()
                    });
                    commands.spawn(PbrBundle {
                        mesh: mesh.clone(),
                        material: materials.choose(&mut material_rng).unwrap().clone(),
                        transform: Transform::from_xyz((x as f32) * 2.5, 0.0, (y as f32) * 2.5),
                        ..default()
                    });
                    commands.spawn(PbrBundle {
                        mesh: mesh.clone(),
                        material: materials.choose(&mut material_rng).unwrap().clone(),
                        transform: Transform::from_xyz(0.0, (x as f32) * 2.5, (y as f32) * 2.5),
                        ..default()
                    });
                }
            }
            // camera
            commands.spawn(Camera3dBundle {
                transform: Transform::from_xyz(WIDTH as f32, HEIGHT as f32, WIDTH as f32),
                ..default()
            });
        }
    }

    commands.spawn(DirectionalLightBundle { ..default() });
}

fn init_textures(args: &Args, images: &mut Assets<Image>) -> Vec<Handle<Image>> {
    let mut color_rng = StdRng::seed_from_u64(42);
    let color_bytes: Vec<u8> = (0..(args.material_texture_count * 4))
        .map(|i| if (i % 4) == 3 { 255 } else { color_rng.gen() })
        .collect();
    color_bytes
        .chunks(4)
        .map(|pixel| {
            images.add(Image::new_fill(
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                pixel,
                TextureFormat::Rgba8UnormSrgb,
            ))
        })
        .collect()
}

fn init_materials(
    args: &Args,
    textures: &[Handle<Image>],
    assets: &mut Assets<StandardMaterial>,
) -> Vec<Handle<StandardMaterial>> {
    let capacity = if args.vary_per_instance {
        match args.layout {
            Layout::Cube => (WIDTH - WIDTH / 10) * (HEIGHT - HEIGHT / 10),
            Layout::Sphere => WIDTH * HEIGHT * 4,
        }
    } else {
        args.material_texture_count
    }
    .max(1);

    let mut materials = Vec::with_capacity(capacity);
    materials.push(assets.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: textures.get(0).cloned(),
        ..default()
    }));

    let mut color_rng = StdRng::seed_from_u64(42);
    let mut texture_rng = StdRng::seed_from_u64(42);
    materials.extend(
        std::iter::repeat_with(|| {
            assets.add(StandardMaterial {
                base_color: Color::rgb_u8(color_rng.gen(), color_rng.gen(), color_rng.gen()),
                base_color_texture: textures.choose(&mut texture_rng).cloned(),
                ..default()
            })
        })
        .take(capacity - materials.len()),
    );

    materials
}

// NOTE: This epsilon value is apparently optimal for optimizing for the average
// nearest-neighbor distance. See:
// http://extremelearning.com.au/how-to-evenly-distribute-points-on-a-sphere-more-effectively-than-the-canonical-fibonacci-lattice/
// for details.
const EPSILON: f64 = 0.36;

fn fibonacci_spiral_on_sphere(golden_ratio: f64, i: usize, n: usize) -> DVec2 {
    DVec2::new(
        PI * 2. * (i as f64 / golden_ratio),
        (1.0 - 2.0 * (i as f64 + EPSILON) / (n as f64 - 1.0 + 2.0 * EPSILON)).acos(),
    )
}

fn spherical_polar_to_cartesian(p: DVec2) -> DVec3 {
    let (sin_theta, cos_theta) = p.x.sin_cos();
    let (sin_phi, cos_phi) = p.y.sin_cos();
    DVec3::new(cos_theta * sin_phi, sin_theta * sin_phi, cos_phi)
}

// System for rotating the camera
fn move_camera(
    time: Res<Time>,
    args: Res<Args>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    let mut camera_transform = camera_query.single_mut();
    let delta = 0.15
        * if args.benchmark {
            1.0 / 60.0
        } else {
            time.delta_seconds()
        };
    camera_transform.rotate_z(delta);
    camera_transform.rotate_x(delta);
}

// System for printing the number of meshes on every tick of the timer
fn print_mesh_count(
    time: Res<Time>,
    mut timer: Local<PrintingTimer>,
    sprites: Query<(&Handle<Mesh>, &ViewVisibility)>,
) {
    timer.tick(time.delta());

    if timer.just_finished() {
        info!(
            "Meshes: {} - Visible Meshes {}",
            sprites.iter().len(),
            sprites.iter().filter(|(_, vis)| vis.get()).count(),
        );
    }
}

#[derive(Deref, DerefMut)]
struct PrintingTimer(Timer);

impl Default for PrintingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}
