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
    asset::RenderAssetUsages,
    camera::visibility::{NoCpuCulling, NoFrustumCulling},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    light::NotShadowCaster,
    math::{DVec2, DVec3},
    prelude::*,
    render::{
        batching::NoAutomaticBatching,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::NoIndirectDrawing,
    },
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};
use rand::{seq::IndexedRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

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
    vary_material_data_per_instance: bool,

    /// the number of different textures from which to randomly select the material base color. 0 means no textures.
    #[argh(option, default = "0")]
    material_texture_count: usize,

    /// the number of different meshes from which to randomly select. Clamped to at least 1.
    #[argh(option, default = "1")]
    mesh_count: usize,

    /// whether to disable all frustum culling. Stresses queuing and batching as all mesh material entities in the scene are always drawn.
    #[argh(switch)]
    no_frustum_culling: bool,

    /// whether to disable automatic batching. Skips batching resulting in heavy stress on render pass draw command encoding.
    #[argh(switch)]
    no_automatic_batching: bool,

    /// whether to disable indirect drawing.
    #[argh(switch)]
    no_indirect_drawing: bool,

    /// whether to disable CPU culling.
    #[argh(switch)]
    no_cpu_culling: bool,

    /// whether to enable directional light cascaded shadow mapping.
    #[argh(switch)]
    shadows: bool,

    /// animate the cube materials by updating the material from the cpu each frame
    #[argh(switch)]
    animate_materials: bool,
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
                "Unknown layout value: '{s}', valid options: 'cube', 'sphere'"
            )),
        }
    }
}

fn main() {
    // `from_env` panics on the web
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(1920.0, 1080.0).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
    ))
    .insert_resource(WinitSettings {
        focused_mode: UpdateMode::Continuous,
        unfocused_mode: UpdateMode::Continuous,
    })
    .add_systems(Startup, setup)
    .add_systems(Update, (move_camera, print_mesh_count));

    if args.animate_materials {
        app.add_systems(Update, update_materials);
    }

    app.insert_resource(args).run();
}

const WIDTH: usize = 200;
const HEIGHT: usize = 200;

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    mesh_assets: ResMut<Assets<Mesh>>,
    material_assets: ResMut<Assets<StandardMaterial>>,
    images: ResMut<Assets<Image>>,
) {
    warn!(include_str!("warning_string.txt"));

    let args = args.into_inner();
    let images = images.into_inner();
    let material_assets = material_assets.into_inner();
    let mesh_assets = mesh_assets.into_inner();

    let meshes = init_meshes(args, mesh_assets);

    let material_textures = init_textures(args, images);
    let materials = init_materials(args, &material_textures, material_assets);

    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let mut material_rng = ChaCha8Rng::seed_from_u64(42);
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
                let (mesh, transform) = meshes.choose(&mut material_rng).unwrap();
                commands
                    .spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(materials.choose(&mut material_rng).unwrap().clone()),
                        Transform::from_translation((radius * unit_sphere_p).as_vec3())
                            .looking_at(Vec3::ZERO, Vec3::Y)
                            .mul_transform(*transform),
                    ))
                    .insert_if(NoFrustumCulling, || args.no_frustum_culling)
                    .insert_if(NoAutomaticBatching, || args.no_automatic_batching);
            }

            // camera
            let mut camera = commands.spawn(Camera3d::default());
            if args.no_indirect_drawing {
                camera.insert(NoIndirectDrawing);
            }
            if args.no_cpu_culling {
                camera.insert(NoCpuCulling);
            }

            // Inside-out box around the meshes onto which shadows are cast (though you cannot see them...)
            commands.spawn((
                Mesh3d(mesh_assets.add(Cuboid::from_size(Vec3::splat(radius as f32 * 2.2)))),
                MeshMaterial3d(material_assets.add(StandardMaterial::from(Color::WHITE))),
                Transform::from_scale(-Vec3::ONE),
                NotShadowCaster,
            ));
        }
        _ => {
            // NOTE: This pattern is good for demonstrating that frustum culling is working correctly
            // as the number of visible meshes rises and falls depending on the viewing angle.
            let scale = 2.5;
            for x in 0..WIDTH {
                for y in 0..HEIGHT {
                    // introduce spaces to break any kind of moiré pattern
                    if x % 10 == 0 || y % 10 == 0 {
                        continue;
                    }
                    // cube
                    commands.spawn((
                        Mesh3d(meshes.choose(&mut material_rng).unwrap().0.clone()),
                        MeshMaterial3d(materials.choose(&mut material_rng).unwrap().clone()),
                        Transform::from_xyz((x as f32) * scale, (y as f32) * scale, 0.0),
                    ));
                    commands.spawn((
                        Mesh3d(meshes.choose(&mut material_rng).unwrap().0.clone()),
                        MeshMaterial3d(materials.choose(&mut material_rng).unwrap().clone()),
                        Transform::from_xyz(
                            (x as f32) * scale,
                            HEIGHT as f32 * scale,
                            (y as f32) * scale,
                        ),
                    ));
                    commands.spawn((
                        Mesh3d(meshes.choose(&mut material_rng).unwrap().0.clone()),
                        MeshMaterial3d(materials.choose(&mut material_rng).unwrap().clone()),
                        Transform::from_xyz((x as f32) * scale, 0.0, (y as f32) * scale),
                    ));
                    commands.spawn((
                        Mesh3d(meshes.choose(&mut material_rng).unwrap().0.clone()),
                        MeshMaterial3d(materials.choose(&mut material_rng).unwrap().clone()),
                        Transform::from_xyz(0.0, (x as f32) * scale, (y as f32) * scale),
                    ));
                }
            }
            // camera
            let center = 0.5 * scale * Vec3::new(WIDTH as f32, HEIGHT as f32, WIDTH as f32);
            commands.spawn((Camera3d::default(), Transform::from_translation(center)));
            // Inside-out box around the meshes onto which shadows are cast (though you cannot see them...)
            commands.spawn((
                Mesh3d(mesh_assets.add(Cuboid::from_size(2.0 * 1.1 * center))),
                MeshMaterial3d(material_assets.add(StandardMaterial::from(Color::WHITE))),
                Transform::from_scale(-Vec3::ONE).with_translation(center),
                NotShadowCaster,
            ));
        }
    }

    commands.spawn((
        DirectionalLight {
            shadows_enabled: args.shadows,
            ..default()
        },
        Transform::IDENTITY.looking_at(Vec3::new(0.0, -1.0, -1.0), Vec3::Y),
    ));
}

fn init_textures(args: &Args, images: &mut Assets<Image>) -> Vec<Handle<Image>> {
    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let mut color_rng = ChaCha8Rng::seed_from_u64(42);
    let color_bytes: Vec<u8> = (0..(args.material_texture_count * 4))
        .map(|i| {
            if (i % 4) == 3 {
                255
            } else {
                color_rng.random()
            }
        })
        .collect();
    color_bytes
        .chunks(4)
        .map(|pixel| {
            images.add(Image::new_fill(
                Extent3d::default(),
                TextureDimension::D2,
                pixel,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            ))
        })
        .collect()
}

fn init_materials(
    args: &Args,
    textures: &[Handle<Image>],
    assets: &mut Assets<StandardMaterial>,
) -> Vec<Handle<StandardMaterial>> {
    let capacity = if args.vary_material_data_per_instance {
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
        base_color_texture: textures.first().cloned(),
        ..default()
    }));

    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let mut color_rng = ChaCha8Rng::seed_from_u64(42);
    let mut texture_rng = ChaCha8Rng::seed_from_u64(42);
    materials.extend(
        std::iter::repeat_with(|| {
            assets.add(StandardMaterial {
                base_color: Color::srgb_u8(
                    color_rng.random(),
                    color_rng.random(),
                    color_rng.random(),
                ),
                base_color_texture: textures.choose(&mut texture_rng).cloned(),
                ..default()
            })
        })
        .take(capacity - materials.len()),
    );

    materials
}

fn init_meshes(args: &Args, assets: &mut Assets<Mesh>) -> Vec<(Handle<Mesh>, Transform)> {
    let capacity = args.mesh_count.max(1);

    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let mut radius_rng = ChaCha8Rng::seed_from_u64(42);
    let mut variant = 0;
    std::iter::repeat_with(|| {
        let radius = radius_rng.random_range(0.25f32..=0.75f32);
        let (handle, transform) = match variant % 15 {
            0 => (
                assets.add(Cuboid {
                    half_size: Vec3::splat(radius),
                }),
                Transform::IDENTITY,
            ),
            1 => (
                assets.add(Capsule3d {
                    radius,
                    half_length: radius,
                }),
                Transform::IDENTITY,
            ),
            2 => (
                assets.add(Circle { radius }),
                Transform::IDENTITY.looking_at(Vec3::Z, Vec3::Y),
            ),
            3 => {
                let mut vertices = [Vec2::ZERO; 3];
                let dtheta = std::f32::consts::TAU / 3.0;
                for (i, vertex) in vertices.iter_mut().enumerate() {
                    let (s, c) = ops::sin_cos(i as f32 * dtheta);
                    *vertex = Vec2::new(c, s) * radius;
                }
                (
                    assets.add(Triangle2d { vertices }),
                    Transform::IDENTITY.looking_at(Vec3::Z, Vec3::Y),
                )
            }
            4 => (
                assets.add(Rectangle {
                    half_size: Vec2::splat(radius),
                }),
                Transform::IDENTITY.looking_at(Vec3::Z, Vec3::Y),
            ),
            v if (5..=8).contains(&v) => (
                assets.add(RegularPolygon {
                    circumcircle: Circle { radius },
                    sides: v,
                }),
                Transform::IDENTITY.looking_at(Vec3::Z, Vec3::Y),
            ),
            9 => (
                assets.add(Cylinder {
                    radius,
                    half_height: radius,
                }),
                Transform::IDENTITY,
            ),
            10 => (
                assets.add(Ellipse {
                    half_size: Vec2::new(radius, 0.5 * radius),
                }),
                Transform::IDENTITY.looking_at(Vec3::Z, Vec3::Y),
            ),
            11 => (
                assets.add(
                    Plane3d {
                        normal: Dir3::NEG_Z,
                        half_size: Vec2::splat(0.5),
                    }
                    .mesh()
                    .size(radius, radius),
                ),
                Transform::IDENTITY,
            ),
            12 => (assets.add(Sphere { radius }), Transform::IDENTITY),
            13 => (
                assets.add(Torus {
                    minor_radius: 0.5 * radius,
                    major_radius: radius,
                }),
                Transform::IDENTITY.looking_at(Vec3::Y, Vec3::Y),
            ),
            14 => (
                assets.add(Capsule2d {
                    radius,
                    half_length: radius,
                }),
                Transform::IDENTITY.looking_at(Vec3::Z, Vec3::Y),
            ),
            _ => unreachable!(),
        };
        variant += 1;
        (handle, transform)
    })
    .take(capacity)
    .collect()
}

// NOTE: This epsilon value is apparently optimal for optimizing for the average
// nearest-neighbor distance. See:
// http://extremelearning.com.au/how-to-evenly-distribute-points-on-a-sphere-more-effectively-than-the-canonical-fibonacci-lattice/
// for details.
const EPSILON: f64 = 0.36;

fn fibonacci_spiral_on_sphere(golden_ratio: f64, i: usize, n: usize) -> DVec2 {
    DVec2::new(
        PI * 2. * (i as f64 / golden_ratio),
        f64::acos(1.0 - 2.0 * (i as f64 + EPSILON) / (n as f64 - 1.0 + 2.0 * EPSILON)),
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
    mut camera_transform: Single<&mut Transform, With<Camera>>,
) {
    let delta = 0.15
        * if args.benchmark {
            1.0 / 60.0
        } else {
            time.delta_secs()
        };
    camera_transform.rotate_z(delta);
    camera_transform.rotate_x(delta);
}

// System for printing the number of meshes on every tick of the timer
fn print_mesh_count(
    time: Res<Time>,
    mut timer: Local<PrintingTimer>,
    sprites: Query<(&Mesh3d, &ViewVisibility)>,
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

fn update_materials(mut materials: ResMut<Assets<StandardMaterial>>, time: Res<Time>) {
    let elapsed = time.elapsed_secs();
    for (i, (_, material)) in materials.iter_mut().enumerate() {
        let hue = (elapsed + i as f32 * 0.005).rem_euclid(1.0);
        // This is much faster than using base_color.set_hue(hue), and in a tight loop it shows.
        let color = fast_hue_to_rgb(hue);
        material.base_color = Color::linear_rgb(color.x, color.y, color.z);
    }
}

#[inline]
fn fast_hue_to_rgb(hue: f32) -> Vec3 {
    (hue * 6.0 - vec3(3.0, 2.0, 4.0)).abs() * vec3(1.0, -1.0, -1.0) + vec3(-1.0, 2.0, 2.0)
}
