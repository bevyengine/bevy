// Press B for benchmark.
// Preferably after frame time is reading consistently, rust-analyzer has calmed down, and with locked gpu clocks.

use std::{f32::consts::PI, time::Instant};

use crate::light_consts::lux;
use argh::FromArgs;
use bevy::pbr::ContactShadows;
use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::{
        visibility::{NoCpuCulling, NoFrustumCulling},
        Hdr,
    },
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass},
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    light::{CascadeShadowConfig, CascadeShadowConfigBuilder},
    pbr::{DefaultOpaqueRendererMethod, ScreenSpaceAmbientOcclusion},
    post_process::bloom::Bloom,
    prelude::*,
    render::{
        batching::NoAutomaticBatching,
        occlusion_culling::OcclusionCulling,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::NoIndirectDrawing,
    },
    scene::SceneInstanceReady,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};

#[derive(FromArgs, Resource, Clone)]
/// Config
pub struct Args {
    /// disable bloom, AO, AA, shadows
    #[argh(switch)]
    minimal: bool,

    /// assign randomly generated materials to each unique mesh (mesh instances also share materials)
    #[argh(switch)]
    random_materials: bool,

    /// quantity of unique textures sets to randomly select from. (A texture set being: base_color, roughness)
    #[argh(option, default = "0")]
    texture_count: u32,

    /// quantity of hotel 01 models
    #[argh(option, default = "1")]
    count: u32,

    /// use deferred shading
    #[argh(switch)]
    deferred: bool,

    /// disable all frustum culling. Stresses queuing and batching as all mesh material entities in the scene are always drawn.
    #[argh(switch)]
    no_frustum_culling: bool,

    /// disable automatic batching. Skips batching resulting in heavy stress on render pass draw command encoding.
    #[argh(switch)]
    no_automatic_batching: bool,

    /// disable gpu occlusion culling for the camera
    #[argh(switch)]
    no_view_occlusion_culling: bool,

    /// disable gpu occlusion culling for the directional light
    #[argh(switch)]
    no_shadow_occlusion_culling: bool,

    /// disable indirect drawing.
    #[argh(switch)]
    no_indirect_drawing: bool,

    /// disable CPU culling.
    #[argh(switch)]
    no_cpu_culling: bool,

    /// spin the bistros and camera
    #[argh(switch)]
    spin: bool,

    /// don't show frame time
    #[argh(switch)]
    hide_frame_time: bool,
}

pub fn main() {
    let args: Args = argh::from_env();

    let mut app = App::new();

    app.init_resource::<CameraPositions>()
        .init_resource::<FrameLowHigh>()
        .insert_resource(args.clone())
        .insert_resource(WinitSettings::continuous())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::Immediate,
                resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            FrameTimeDiagnosticsPlugin {
                max_history_length: 1000,
                ..default()
            },
            FreeCameraPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (input, spin, frame_time_system, benchmark).chain());

    if args.no_frustum_culling {
        app.add_systems(Update, add_no_frustum_culling);
    }

    if args.deferred {
        app.insert_resource(DefaultOpaqueRendererMethod::deferred());
    }

    app.run();
}

#[derive(Component)]
pub struct Spin;

#[derive(Component)]
struct FrameTimeText;

#[derive(Component)]
pub struct PostProcScene;

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    args: Res<Args>,
    positions: Res<CameraPositions>,
) {
    let hotel_01 = asset_server.load("hotel_01.glb#Scene0");
    commands
        .spawn((
            SceneRoot(hotel_01.clone()),
            Transform::from_scale(Vec3::splat(0.01)),
            PostProcScene,
            Spin,
        ))
        .observe(assign_rng_materials);

    let mut count = 0;
    if args.count > 1 {
        let quantity = args.count - 1;
        let side = (quantity as f32).sqrt().ceil() as i32 / 2;
        'outer: for x in -side..=side {
            for z in -side..=side {
                if count >= quantity {
                    break 'outer;
                }
                if x == 0 && z == 0 {
                    continue;
                }
                commands.spawn((
                    SceneRoot(hotel_01.clone()),
                    Transform::from_xyz(x as f32 * 50.0, 0.0, z as f32 * 50.0)
                        .with_scale(Vec3::splat(0.01)),
                    Spin,
                ));
                count += 1;
            }
        }
    }

    // Sun
    commands
        .spawn((
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI * -0.35, PI * -0.13, 0.0)),
            DirectionalLight {
                color: Color::srgb(1.0, 0.87, 0.78),
                illuminance: lux::FULL_DAYLIGHT,
                shadow_maps_enabled: !args.minimal,
                contact_shadows_enabled: !args.minimal,
                shadow_depth_bias: 0.2,
                shadow_normal_bias: 0.2,
                ..default()
            },
            CascadeShadowConfig::from(CascadeShadowConfigBuilder {
                num_cascades: 3,
                minimum_distance: 0.1,
                maximum_distance: 80.0,
                first_cascade_far_bound: 5.0,
                overlap_proportion: 0.2,
            }),
        ))
        .insert_if(OcclusionCulling, || !args.no_shadow_occlusion_culling);

    // Camera
    let mut cam = commands.spawn((
        Msaa::Off,
        Camera3d::default(),
        Hdr,
        positions[0],
        Projection::Perspective(PerspectiveProjection {
            fov: std::f32::consts::PI / 3.0,
            near: 0.1,
            far: 1000.0,
            ..Default::default()
        }),
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 1000.0,
            ..default()
        },
        ContactShadows::default(),
        FreeCamera::default(),
        Spin,
    ));

    cam.insert_if(DepthPrepass, || args.deferred)
        .insert_if(DeferredPrepass, || args.deferred)
        .insert_if(OcclusionCulling, || !args.no_view_occlusion_culling)
        .insert_if(NoFrustumCulling, || args.no_frustum_culling)
        .insert_if(NoAutomaticBatching, || args.no_automatic_batching)
        .insert_if(NoIndirectDrawing, || args.no_indirect_drawing)
        .insert_if(NoCpuCulling, || args.no_cpu_culling);

    if !args.minimal {
        cam.insert((
            Bloom {
                intensity: 0.02,
                ..default()
            },
            TemporalAntiAliasing::default(),
        ))
        .insert(ScreenSpaceAmbientOcclusion::default());
    }

    if !args.hide_frame_time {
        commands
            .spawn((
                Node {
                    left: Val::Px(1.5),
                    top: Val::Px(1.5),
                    ..default()
                },
                GlobalZIndex(-1),
            ))
            .with_children(|parent| {
                parent.spawn((Text::new(""), TextColor(Color::BLACK), FrameTimeText));
            });
        commands.spawn(Node::default()).with_children(|parent| {
            parent.spawn((Text::new(""), TextColor(Color::WHITE), FrameTimeText));
        });
    }
}

// Go though each unique mesh and randomly generate a material.
// Each unique so instances are maintained.
#[allow(clippy::too_many_arguments)]
pub fn assign_rng_materials(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    meshes: Res<Assets<Mesh>>,
    mesh_instances: Query<(Entity, &Mesh3d)>,
    args: Res<Args>,
    asset_server: Res<AssetServer>,
    scenes: Query<&SceneRoot>,
) {
    if !args.random_materials {
        return;
    }

    let Ok(scene) = scenes.get(scene_ready.entity) else {
        return;
    };

    let scene_loaded = asset_server
        .get_recursive_dependency_load_state(&scene.0)
        .map(|state| state.is_loaded())
        .unwrap_or(false);

    if !scene_loaded {
        warn!("get_recursive_dependency_load_state not finished!");
    }

    const MESH_INSTANCE_QTY: usize = 35689;
    if MESH_INSTANCE_QTY != mesh_instances.iter().len() {
        warn!(
            "Mesh quantity appears incorrect. Expected: {}. Found: {}!",
            MESH_INSTANCE_QTY,
            mesh_instances.iter().len()
        )
    }

    let base_color_textures = (0..args.texture_count)
        .map(|i| {
            images.add(generate_random_compressed_texture_with_mipmaps(
                2048, false, i,
            ))
        })
        .collect::<Vec<_>>();
    let roughness_textures = (0..args.texture_count)
        .map(|i| {
            images.add(generate_random_compressed_texture_with_mipmaps(
                2048,
                false, // Using bc4 here seems to not work
                i + 2048,
            ))
        })
        .collect::<Vec<_>>();

    for (i, (mesh_h, _mesh)) in meshes.iter().enumerate() {
        let mut base_color_texture = None;
        let mut roughness_texture = None;

        if !base_color_textures.is_empty() {
            base_color_texture = Some(base_color_textures[i % base_color_textures.len()].clone());
        }
        if !roughness_textures.is_empty() {
            roughness_texture = Some(roughness_textures[i % roughness_textures.len()].clone());
        }

        let unique_material = materials.add(StandardMaterial {
            base_color: Color::srgb(
                hash_noise(i as u32, 0, 0),
                hash_noise(i as u32, 0, 1),
                hash_noise(i as u32, 0, 2),
            ),
            base_color_texture,
            metallic_roughness_texture: roughness_texture,
            ..default()
        });
        for (entity, mesh_instance_h) in mesh_instances.iter() {
            if mesh_instance_h.id() == mesh_h {
                commands
                    .entity(entity)
                    .insert(MeshMaterial3d::from(unique_material.clone()));
            }
        }
    }
}

fn generate_random_compressed_texture_with_mipmaps(size: u32, bc4: bool, seed: u32) -> Image {
    let (bytes, mip_count) = calculate_bcn_image_size_with_mips(size, if bc4 { 8 } else { 16 });
    let data = (0..bytes).map(|i| uhash(i, seed) as u8).collect::<Vec<_>>();

    Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size,
                height: size,
                ..default()
            },
            dimension: TextureDimension::D2,
            format: if bc4 {
                TextureFormat::Bc4RUnorm
            } else {
                TextureFormat::Bc7RgbaUnormSrgb
            },
            mip_level_count: mip_count,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            ..default()
        }),

        data: Some(data),
        ..Default::default()
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct CameraPositions([Transform; 3]);

impl Default for CameraPositions {
    fn default() -> Self {
        Self([
            Transform {
                translation: Vec3::new(-20.147331, 16.818098, 42.806145),
                rotation: Quat::from_array([-0.22917402, -0.34915298, -0.08848568, 0.9042908]),
                scale: Vec3::ONE,
            },
            Transform {
                translation: Vec3::new(1.6168646, 1.8304176, -5.846825),
                rotation: Quat::from_array([-0.0007061247, -0.99179053, 0.12775362, -0.005481863]),
                scale: Vec3::ONE,
            },
            Transform {
                translation: Vec3::new(23.97184, 1.8938808, 30.568554),
                rotation: Quat::from_array([-0.0013945175, 0.4685419, 0.00073959737, 0.8834399]),
                scale: Vec3::ONE,
            },
        ])
    }
}

fn input(
    input: Res<ButtonInput<KeyCode>>,
    mut camera: Query<&mut Transform, With<Camera>>,
    positions: Res<CameraPositions>,
) {
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };
    if input.just_pressed(KeyCode::KeyI) {
        info!("{:?}", transform);
    }
    if input.just_pressed(KeyCode::Digit1) {
        *transform = positions[0]
    }
    if input.just_pressed(KeyCode::Digit2) {
        *transform = positions[1]
    }
    if input.just_pressed(KeyCode::Digit3) {
        *transform = positions[2]
    }
}

fn spin(
    camera: Single<Entity, With<Camera>>,
    mut things_to_spin: Query<&mut Transform, With<Spin>>,
    time: Res<Time>,
    args: Res<Args>,
    mut positions: ResMut<CameraPositions>,
) {
    if args.spin {
        let camera_position = things_to_spin.get(*camera).unwrap().translation;
        let spin = |thing_to_spin: &mut Transform| {
            thing_to_spin.rotate_around(camera_position, Quat::from_rotation_y(time.delta_secs()));
        };
        things_to_spin.iter_mut().for_each(|mut s| spin(s.as_mut())); // WHY
        positions.iter_mut().for_each(spin);
    }
}

#[allow(clippy::too_many_arguments)]
fn benchmark(
    input: Res<ButtonInput<KeyCode>>,
    mut camera_transform: Single<&mut Transform, With<Camera>>,
    materials: Res<Assets<StandardMaterial>>,
    meshes: Res<Assets<Mesh>>,
    has_std_mat: Query<&MeshMaterial3d<StandardMaterial>>,
    has_mesh: Query<&Mesh3d>,
    mut bench_started: Local<Option<Instant>>,
    mut bench_frame: Local<u32>,
    mut count_per_step: Local<u32>,
    time: Res<Time>,
    positions: Res<CameraPositions>,
    mut low_high: ResMut<FrameLowHigh>,
) {
    if input.just_pressed(KeyCode::KeyB) && bench_started.is_none() {
        low_high.bench_reset();
        *bench_started = Some(Instant::now());
        *bench_frame = 0;
        // Try to render for around 3s or at least 60 frames per step
        *count_per_step = ((3.0 / time.delta_secs()) as u32).max(60);
        println!(
            "Starting Benchmark with {} frames per step",
            *count_per_step
        );
    }
    if bench_started.is_none() {
        return;
    }
    if *bench_frame == 0 {
        **camera_transform = positions[0]
    } else if *bench_frame == *count_per_step {
        **camera_transform = positions[1]
    } else if *bench_frame == *count_per_step * 2 {
        **camera_transform = positions[2]
    } else if *bench_frame == *count_per_step * 3 {
        let elapsed = bench_started.unwrap().elapsed().as_secs_f32();
        println!(
            "{:>7.2}ms Benchmark avg cpu frame time",
            (elapsed / *bench_frame as f32) * 1000.0
        );
        let r = 1.0 / *bench_frame as f64;
        println!("{:>7.2}ms avg 1% low", low_high.sum_one_percent_low * r);
        println!("{:>7.2}ms avg 1% high", low_high.sum_one_percent_high * r);
        println!(
            "{:>7} Meshes\n{:>7} Mesh Instances\n{:>7} Materials\n{:>7} Material Instances",
            meshes.len(),
            has_mesh.iter().len(),
            materials.len(),
            has_std_mat.iter().len(),
        );
        *bench_started = None;
        *bench_frame = 0;
        **camera_transform = positions[0];
    }
    *bench_frame += 1;
    low_high.bench_step();
}

pub fn add_no_frustum_culling(
    mut commands: Commands,
    convert_query: Query<
        Entity,
        (
            Without<NoFrustumCulling>,
            With<MeshMaterial3d<StandardMaterial>>,
        ),
    >,
) {
    for entity in convert_query.iter() {
        commands.entity(entity).insert(NoFrustumCulling);
    }
}

#[inline(always)]
pub fn uhash(a: u32, b: u32) -> u32 {
    let mut x = (a.overflowing_mul(1597334673).0) ^ (b.overflowing_mul(3812015801).0);
    // from https://nullprogram.com/blog/2018/07/31/
    x = x ^ (x >> 16);
    x = x.overflowing_mul(0x7feb352d).0;
    x = x ^ (x >> 15);
    x = x.overflowing_mul(0x846ca68b).0;
    x = x ^ (x >> 16);
    x
}

#[inline(always)]
pub fn unormf(n: u32) -> f32 {
    n as f32 * (1.0 / 0xffffffffu32 as f32)
}

#[inline(always)]
pub fn hash_noise(x: u32, y: u32, z: u32) -> f32 {
    let urnd = uhash(x, (y << 11) + z);
    unormf(urnd)
}

// BC7 block is 16 bytes, BC4 block is 8 bytes
fn calculate_bcn_image_size_with_mips(size: u32, block_size: u32) -> (u32, u32) {
    let mut total_size = 0;
    let mut mip_size = size;
    let mut mip_count = 0;
    while mip_size > 4 {
        mip_count += 1;
        let num_blocks = mip_size / 4; // Round up
        let mip_level_size = num_blocks * num_blocks * block_size;
        total_size += mip_level_size;
        mip_size = (mip_size / 2).max(1);
    }
    (total_size, mip_count.max(1))
}

#[derive(Resource, Default)]
struct FrameLowHigh {
    one_percent_low: f64,
    one_percent_high: f64,
    sum_one_percent_low: f64,
    sum_one_percent_high: f64,
}

impl FrameLowHigh {
    fn bench_reset(&mut self) {
        self.sum_one_percent_high = 0.0;
        self.sum_one_percent_low = 0.0;
    }
    fn bench_step(&mut self) {
        self.sum_one_percent_high += self.one_percent_high;
        self.sum_one_percent_low += self.one_percent_low;
    }
}

fn frame_time_system(
    diagnostics: Res<DiagnosticsStore>,
    mut text: Query<&mut Text, With<FrameTimeText>>,
    mut measurements: Local<Vec<f64>>,
    mut low_high: ResMut<FrameLowHigh>,
) {
    if let Some(frame_time) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME) {
        let mut string = format!(
            "\n{:>7.2}ms ema\n{:>7.2}ms sma\n",
            frame_time.smoothed().unwrap_or_default(),
            frame_time.average().unwrap_or_default()
        );

        if frame_time.history_len() >= 100 {
            measurements.clear();
            measurements.extend(frame_time.measurements().map(|t| t.value));
            measurements.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let count = measurements.len() / 100;
            low_high.one_percent_low = measurements.iter().take(count).sum::<f64>() / count as f64;
            low_high.one_percent_high =
                measurements.iter().rev().take(count).sum::<f64>() / count as f64;

            string.push_str(&format!(
                "{:>7.2}ms 1% low\n{:>7.2}ms 1% high\n",
                low_high.one_percent_low, low_high.one_percent_high
            ));
        }

        for mut t in &mut text {
            t.0 = string.clone();
        }
    };
}
