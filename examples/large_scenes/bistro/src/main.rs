// Press B for benchmark.
// Preferably after frame time is reading consistently, rust-analyzer has calmed down, and with locked gpu clocks.

use std::{
    f32::consts::PI,
    ops::{Add, Mul, Sub},
    path::PathBuf,
    time::Instant,
};

use argh::FromArgs;
use bevy::pbr::ContactShadows;
use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::visibility::{NoCpuCulling, NoFrustumCulling},
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass},
    diagnostic::DiagnosticsStore,
    light::TransmittedShadowReceiver,
    pbr::{
        DefaultOpaqueRendererMethod, ScreenSpaceAmbientOcclusion, ScreenSpaceTransmission,
        ScreenSpaceTransmissionQuality,
    },
    post_process::bloom::Bloom,
    render::{
        batching::NoAutomaticBatching, occlusion_culling::OcclusionCulling, render_resource::Face,
        view::NoIndirectDrawing,
    },
    scene::SceneInstanceReady,
};
use bevy::{
    camera::Hdr,
    diagnostic::FrameTimeDiagnosticsPlugin,
    light::CascadeShadowConfigBuilder,
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use mipmap_generator::{
    generate_mipmaps, MipmapGeneratorDebugTextPlugin, MipmapGeneratorPlugin,
    MipmapGeneratorSettings,
};

use crate::light_consts::lux;

#[derive(FromArgs, Resource, Clone)]
/// Config
pub struct Args {
    /// disable glTF lights
    #[argh(switch)]
    no_gltf_lights: bool,

    /// disable bloom, AO, AA, shadows
    #[argh(switch)]
    minimal: bool,

    /// compress textures (if they are not already, requires compress feature)
    #[argh(switch)]
    compress: bool,

    /// if low_quality_compression is set, only 0.5 byte/px formats will be used (BC1, BC4) unless the alpha channel is in use, then BC3 will be used.
    /// When low quality is set, compression is generally faster than CompressionSpeed::UltraFast and CompressionSpeed is ignored.
    #[argh(switch)]
    low_quality_compression: bool,

    /// compressed texture cache (requires compress feature)
    #[argh(switch)]
    cache: bool,

    /// quantity of bistros
    #[argh(option, default = "1")]
    count: u32,

    /// spin the bistros and camera
    #[argh(switch)]
    spin: bool,

    /// don't show frame time
    #[argh(switch)]
    hide_frame_time: bool,

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
}

pub fn main() {
    let args: Args = argh::from_env();

    let mut app = App::new();

    app.init_resource::<CameraPositions>()
        .init_resource::<FrameLowHigh>()
        .insert_resource(GlobalAmbientLight::NONE)
        .insert_resource(args.clone())
        .insert_resource(ClearColor(Color::srgb(1.75, 1.9, 1.99)))
        .insert_resource(WinitSettings::continuous())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::Immediate,
                resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }))
        // Generating mipmaps takes a minute
        // Mipmap generation be skipped if ktx2 is used
        .insert_resource(MipmapGeneratorSettings {
            anisotropic_filtering: 16,
            compression: args.compress.then(Default::default),
            compressed_image_data_cache_path: if args.cache {
                Some(PathBuf::from("compressed_texture_cache"))
            } else {
                None
            },
            low_quality: args.low_quality_compression,
            ..default()
        })
        .add_plugins((
            FrameTimeDiagnosticsPlugin {
                max_history_length: 1000,
                ..default()
            },
            MipmapGeneratorPlugin,
            MipmapGeneratorDebugTextPlugin,
            FreeCameraPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                generate_mipmaps::<StandardMaterial>,
                input,
                run_animation,
                spin,
                frame_time_system,
                benchmark,
            )
                .chain(),
        );

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

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>, args: Res<Args>) {
    println!("Loading models, generating mipmaps");

    let bistro_exterior = asset_server.load("bistro_exterior/BistroExterior.gltf#Scene0");
    commands
        .spawn((SceneRoot(bistro_exterior.clone()), Spin))
        .observe(proc_scene);

    let bistro_interior = asset_server.load("bistro_interior_wine/BistroInterior_Wine.gltf#Scene0");
    commands
        .spawn((SceneRoot(bistro_interior.clone()), Spin))
        .observe(proc_scene);

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
                commands
                    .spawn((
                        SceneRoot(bistro_exterior.clone()),
                        Transform::from_xyz(x as f32 * 150.0, 0.0, z as f32 * 150.0),
                        Spin,
                    ))
                    .observe(proc_scene);
                commands
                    .spawn((
                        SceneRoot(bistro_interior.clone()),
                        Transform::from_xyz(x as f32 * 150.0, 0.3, z as f32 * 150.0 - 0.2),
                        Spin,
                    ))
                    .observe(proc_scene);
                count += 1;
            }
        }
    }

    if !args.no_gltf_lights {
        // In Repo glTF
        commands.spawn((
            SceneRoot(asset_server.load("BistroExteriorFakeGI.gltf#Scene0")),
            Spin,
        ));
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
                shadow_depth_bias: 0.1,
                shadow_normal_bias: 0.2,
                ..default()
            },
            CascadeShadowConfigBuilder {
                num_cascades: 3,
                minimum_distance: 0.05,
                maximum_distance: 100.0,
                first_cascade_far_bound: 10.0,
                overlap_proportion: 0.2,
            }
            .build(),
        ))
        .insert_if(OcclusionCulling, || !args.no_shadow_occlusion_culling);

    // Camera
    let mut cam = commands.spawn((
        Msaa::Off,
        Camera3d::default(),
        ScreenSpaceTransmission {
            screen_space_specular_transmission_steps: 0,
            screen_space_specular_transmission_quality: ScreenSpaceTransmissionQuality::Low,
        },
        Hdr,
        Transform::from_xyz(-10.5, 1.7, -1.0).looking_at(Vec3::new(0.0, 3.5, 0.0), Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: std::f32::consts::PI / 3.0,
            near: 0.1,
            far: 1000.0,
            aspect_ratio: 1.0,
            ..Default::default()
        }),
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/san_giuseppe_bridge_4k_diffuse.ktx2"),
            specular_map: asset_server
                .load("environment_maps/san_giuseppe_bridge_4k_specular.ktx2"),
            intensity: 600.0,
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

pub fn all_children<F: FnMut(Entity)>(
    children: &Children,
    children_query: &Query<&Children>,
    closure: &mut F,
) {
    for child in children {
        if let Ok(children) = children_query.get(*child) {
            all_children(children, children_query, closure);
        }
        closure(*child);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn proc_scene(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    has_std_mat: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    lights: Query<Entity, Or<(With<PointLight>, With<DirectionalLight>, With<SpotLight>)>>,
    cameras: Query<Entity, With<Camera>>,
    args: Res<Args>,
) {
    for entity in children.iter_descendants(scene_ready.entity) {
        // Sponza needs flipped normals
        if let Ok(mat_h) = has_std_mat.get(entity)
            && let Some(mat) = materials.get_mut(mat_h)
        {
            mat.flip_normal_map_y = true;
            match mat.alpha_mode {
                AlphaMode::Mask(_) => {
                    mat.diffuse_transmission = 0.6;
                    mat.double_sided = true;
                    mat.cull_mode = None;
                    mat.thickness = 0.2;
                    commands.entity(entity).insert(TransmittedShadowReceiver);
                }
                AlphaMode::Opaque => {
                    mat.double_sided = false;
                    mat.cull_mode = Some(Face::Back);
                }
                _ => (),
            }
        }

        if args.no_gltf_lights {
            // Has a bunch of lights by default
            if lights.get(entity).is_ok() {
                commands.entity(entity).despawn();
            }
        }

        // Has a bunch of cameras by default
        if cameras.get(entity).is_ok() {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
struct CameraPositions([Transform; 3]);

impl Default for CameraPositions {
    fn default() -> Self {
        Self([
            Transform {
                translation: Vec3::new(-10.5, 1.7, -1.0),
                rotation: Quat::from_array([-0.05678932, 0.7372272, -0.062454797, -0.670351]),
                scale: Vec3::ONE,
            },
            Transform {
                translation: Vec3::new(56.23809, 2.9985719, 28.96291),
                rotation: Quat::from_array([0.0020175162, 0.35272083, -0.0007605003, 0.93572617]),
                scale: Vec3::ONE,
            },
            Transform {
                translation: Vec3::new(5.7861176, 3.3475509, -8.821455),
                rotation: Quat::from_array([-0.0049382094, -0.98193514, -0.025878597, 0.18737496]),
                scale: Vec3::ONE,
            },
        ])
    }
}

const ANIM_SPEED: f32 = 0.2;
const ANIM_HYSTERESIS: f32 = 0.1; // EMA/LPF

const ANIM_CAM: [Transform; 3] = [
    Transform {
        translation: Vec3::new(-6.414026, 8.179898, -23.550516),
        rotation: Quat::from_array([-0.016413536, -0.88136566, -0.030704278, 0.4711502]),
        scale: Vec3::ONE,
    },
    Transform {
        translation: Vec3::new(-14.752817, 6.279289, 5.691277),
        rotation: Quat::from_array([-0.031593435, -0.516736, -0.019086324, 0.8553488]),
        scale: Vec3::ONE,
    },
    Transform {
        translation: Vec3::new(5.1539426, 8.142523, 16.436222),
        rotation: Quat::from_array([-0.07907656, -0.07581916, -0.006031934, 0.99396276]),
        scale: Vec3::ONE,
    },
];

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

fn lerp<T>(a: T, b: T, t: f32) -> T
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T>,
{
    a + (b - a) * t
}

fn follow_path(points: &[Transform], progress: f32) -> Transform {
    let total_segments = (points.len() - 1) as f32;
    let progress = progress.clamp(0.0, 1.0);
    let mut segment_progress = progress * total_segments;
    let segment_index = segment_progress.floor() as usize;
    segment_progress -= segment_index as f32;
    let a = points[segment_index];
    let b = points[(segment_index + 1).min(points.len() - 1)];
    Transform {
        translation: lerp(a.translation, b.translation, segment_progress),
        rotation: lerp(a.rotation, b.rotation, segment_progress),
        scale: lerp(a.scale, b.scale, segment_progress),
    }
}

fn run_animation(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut animation_active: Local<bool>,
    mut camera: Query<&mut Transform, With<Camera>>,
) {
    let Ok(mut cam_tr) = camera.single_mut() else {
        return;
    };
    if input.just_pressed(KeyCode::Space) {
        *animation_active = !*animation_active;
    }
    if !*animation_active {
        return;
    }
    let progress = (time.elapsed_secs() * ANIM_SPEED).fract();
    let cycle = 1.0 - (progress * 2.0 - 1.0).abs();
    let path_state = follow_path(&ANIM_CAM, cycle);
    cam_tr.translation = lerp(cam_tr.translation, path_state.translation, ANIM_HYSTERESIS);
    cam_tr.rotation = lerp(cam_tr.rotation, path_state.rotation, ANIM_HYSTERESIS).normalize();
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
