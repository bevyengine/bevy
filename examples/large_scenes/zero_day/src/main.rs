//! Beeple's "Zero-Day" sci-fi corridor (NVIDIA ORCA), path-traced with Bevy Solari.
//!
//! See this example's `README.md` for how to get and convert the scene and for the
//! command-line options.
//!
//! Controls: `C` changes between the film flythrough and free-fly (WASD and mouse), `N`
//! turns DLSS Ray Reconstruction on and off, `B` runs a short benchmark and prints the
//! result to the console.

use std::f32::consts::{PI, TAU};
use std::time::{Duration, Instant};

use argh::FromArgs;
use bevy::{
    asset::LoadState,
    camera::{CameraMainTextureUsages, Hdr},
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    gltf::Gltf,
    math::ops,
    mesh::Indices,
    platform::collections::{HashMap, HashSet},
    post_process::bloom::Bloom,
    prelude::*,
    render::render_resource::TextureUsages,
    solari::prelude::{RaytracingMesh3d, SolariLighting, SolariPlugins},
    time::common_conditions::on_timer,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
    world_serialization::WorldInstanceReady,
};

// DLSS Ray Reconstruction denoises the Solari output. It needs an NVIDIA RTX GPU, but
// the path tracer still runs without one.
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy::{
    anti_alias::dlss::{
        Dlss, DlssPerfQualityMode, DlssProjectId, DlssRayReconstructionFeature,
        DlssRayReconstructionSupported,
    },
    render::camera::{MipBias, TemporalJitter},
};

/// Config
#[derive(FromArgs, Resource)]
pub struct Args {
    /// which scene to load: measure_one (default), measure_seven, or
    /// measure_seven_colored_lights
    #[argh(option, default = "Scene::MeasureOne")]
    scene: Scene,

    /// emissive multiplier for the light panels (default 150000)
    #[argh(option, default = "DEFAULT_EMISSIVE")]
    emissive: f32,

    /// disable the synthetic emissive pulse that substitutes for the film's animated
    /// lights
    #[argh(switch)]
    no_pulse: bool,

    /// render without Solari, with a flat ambient light instead (not representative; for
    /// profiling and smoke tests)
    #[argh(switch)]
    no_solari: bool,

    /// render resolution as `WxH` (default 1920x1080)
    #[argh(option, default = "(1920, 1080)", from_str_fn(parse_resolution))]
    resolution: (u32, u32),

    /// DLSS quality mode: auto (default), dlaa, quality, balanced, performance, or
    /// ultra_performance
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    #[argh(
        option,
        default = "DlssPerfQualityMode::Auto",
        from_str_fn(parse_dlss_quality)
    )]
    dlss_quality: DlssPerfQualityMode,
}

fn parse_resolution(value: &str) -> Result<(u32, u32), String> {
    value
        .split_once(['x', 'X'])
        .and_then(|(w, h)| Some((w.trim().parse().ok()?, h.trim().parse().ok()?)))
        .ok_or_else(|| format!("expected WxH (e.g. 1920x1080), got `{value}`"))
}

#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
fn parse_dlss_quality(value: &str) -> Result<DlssPerfQualityMode, String> {
    match value {
        "auto" => Ok(DlssPerfQualityMode::Auto),
        "dlaa" => Ok(DlssPerfQualityMode::Dlaa),
        "quality" => Ok(DlssPerfQualityMode::Quality),
        "balanced" => Ok(DlssPerfQualityMode::Balanced),
        "performance" => Ok(DlssPerfQualityMode::Performance),
        "ultra_performance" => Ok(DlssPerfQualityMode::UltraPerformance),
        other => Err(format!(
            "unknown DLSS quality `{other}`; expected auto, dlaa, quality, balanced, \
             performance, or ultra_performance"
        )),
    }
}

/// The scene to load. Only the asset filename differs between measures.
// Keep the shared `Measure` prefix, which matches the Zero-Day asset names.
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy)]
enum Scene {
    MeasureOne,
    MeasureSeven,
    MeasureSevenColoredLights,
}

impl Scene {
    fn glb(self) -> &'static str {
        match self {
            Scene::MeasureOne => "zero_day_measure_one.glb",
            Scene::MeasureSeven => "zero_day_measure_seven.glb",
            Scene::MeasureSevenColoredLights => "zero_day_measure_seven_colored_lights.glb",
        }
    }
}

/// Default emissive multiplier, approximately the luminance of a bright LED fixture in
/// nits.
const DEFAULT_EMISSIVE: f32 = 150_000.0;

impl argh::FromArgValue for Scene {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        match value {
            "measure_one" => Ok(Scene::MeasureOne),
            "measure_seven" => Ok(Scene::MeasureSeven),
            "measure_seven_colored_lights" => Ok(Scene::MeasureSevenColoredLights),
            other => Err(format!(
                "unknown scene `{other}`; expected measure_one, measure_seven, or \
                 measure_seven_colored_lights"
            )),
        }
    }
}

// Tuning for the synthetic emissive pulse (see `animate_emissive`).
/// Rate of the wave in time (rad/s).
const PULSE_FREQ: f32 = 2.0;
/// Spatial frequency along the corridor's Z axis (rad/world-unit).
const PULSE_WAVE_NUMBER: f32 = 0.05;
/// Exponent that sharpens the sine into distinct flares.
const PULSE_SHARPNESS: f32 = 2.0;
/// Minimum and maximum levels, as a fraction of each panel's base emissive.
const PULSE_FLOOR: f32 = 0.4;
const PULSE_PEAK: f32 = 1.8;
/// Golden angle (rad), which gives each panel a different but stable phase.
const PULSE_PHASE_STRIDE: f32 = 2.399_963_2;

fn main() {
    let args: Args = argh::from_env();
    let (win_w, win_h) = args.resolution;

    let mut app = App::new();

    // DLSS reads the project ID when the renderer starts, so set it before the plugins.
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    app.insert_resource(DlssProjectId(bevy::asset::uuid::uuid!(
        "b1a7c0de-4d2f-4e6a-9b3c-0d1e2f3a4b5c"
    )));

    let no_solari = args.no_solari;

    app.insert_resource(ClearColor(Color::BLACK))
        // With `--no-solari`, nothing emits light, so a flat ambient substitute keeps
        // the geometry visible.
        .insert_resource(if no_solari {
            GlobalAmbientLight {
                brightness: 5_000.0,
                ..default()
            }
        } else {
            GlobalAmbientLight::NONE
        })
        .insert_resource(args)
        .insert_resource(WinitSettings::continuous())
        .insert_resource(Cinematic { active: true })
        .init_resource::<FilmLength>()
        .init_resource::<FrameStats>()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::Immediate,
                    resolution: WindowResolution::new(win_w, win_h).with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            }),
            FreeCameraPlugin,
            // A long history lets `frame_stats` calculate the slowest 1% for the HUD.
            FrameTimeDiagnosticsPlugin {
                max_history_length: 1000,
                ..default()
            },
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, spawn_scene_when_ready)
        .add_systems(
            Update,
            (
                (toggle_flythrough, drive_flythrough).chain(),
                animate_emissive,
                benchmark,
                // Updating the HUD dirties the UI layout; at 10 Hz that cost stays out
                // of the measured frame times.
                (frame_stats, update_hud)
                    .chain()
                    .run_if(on_timer(Duration::from_millis(100))),
            ),
        );

    // `SolariPlugins` requests the ray-tracing device features, so `--no-solari` must
    // skip the plugin to run on any GPU.
    if !no_solari {
        app.add_plugins(SolariPlugins);
    }

    // Ray Reconstruction does nothing when Solari is off.
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    app.add_systems(Update, toggle_denoiser.run_if(move || !no_solari));

    app.run();
}

// Components and resources

/// The render camera of the example.
#[derive(Component)]
struct RenderCamera;

/// The imported film camera. Only its animated transform is used; `drive_flythrough`
/// follows it.
#[derive(Component)]
struct FilmCamera;

/// The readout on the screen.
#[derive(Component)]
struct HudText;

/// Keeps the full glTF loaded so its animation clips stay available.
#[derive(Resource)]
struct SceneGltf(Handle<Gltf>);

/// One emissive panel instance, with its own material clone so `animate_emissive` can
/// pulse each panel independently.
#[derive(Component)]
struct EmissivePanel {
    /// The increased base emissive. The pulse multiplies this value.
    base: LinearRgba,
    /// The stable phase offset of this panel (radians).
    phase: f32,
}

/// The length of the film animation (seconds), from the longest animation clip. The `B`
/// benchmark uses it to measure exactly one loop.
#[derive(Resource, Default)]
struct FilmLength(f32);

/// Frame-time statistics for the HUD. `one_percent_high_ms` is the mean of the slowest 1%
/// of the frames.
#[derive(Resource, Default)]
struct FrameStats {
    avg_ms: f64,
    one_percent_high_ms: f64,
}

/// Whether the camera follows the film flythrough. The `C` key toggles free-fly.
#[derive(Resource)]
struct Cinematic {
    active: bool,
}

// Setup

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    args: Res<Args>,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] dlss_rr_supported: Option<
        Res<DlssRayReconstructionSupported>,
    >,
) {
    let glb = args.scene.glb();
    println!("Loading Zero-Day `{glb}` (this is a large scene; give it a moment)");

    // Load the full glTF, not only the scene, so the animation clips stay available to
    // `start_animation`.
    commands.insert_resource(SceneGltf(asset_server.load(glb)));

    // The transform below is the view until the flythrough starts.
    // `setup_flythrough_camera` later copies the film camera's field of view and near
    // plane.
    let mut cam = commands.spawn((
        Camera3d::default(),
        // The imported film camera also spawns as an active camera with order 0. The
        // higher order keeps this camera in control until `setup_flythrough_camera`
        // removes the other one.
        Camera {
            order: 1,
            ..default()
        },
        Hdr,
        // Solari and DLSS need MSAA off. It also stays off with `--no-solari` to keep
        // the profiling numbers comparable.
        Msaa::Off,
        Transform::from_xyz(-27.0, 8.0, 70.0).looking_at(Vec3::new(-27.0, 8.0, -150.0), Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: PI / 3.0,
            near: 0.1,
            far: 2000.0,
            ..default()
        }),
        RenderCamera,
        // Glare on the bright panels.
        Bloom {
            intensity: 0.15,
            ..Bloom::NATURAL
        },
        FreeCamera {
            walk_speed: 20.0,
            run_speed: 60.0,
            ..default()
        },
    ));
    cam.insert_if(
        (
            // Solari writes its result into the main texture with a storage binding.
            CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
            SolariLighting::default(),
        ),
        || !args.no_solari,
    );
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if dlss_rr_supported.is_some() && !args.no_solari {
        cam.insert(dlss_rr(args.dlss_quality));
    }

    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: px(8.0),
            left: px(8.0),
            ..default()
        },
        HudText,
    ));
}

/// Spawns the scene once its glTF and all of its dependencies are loaded, so that the
/// `WorldInstanceReady` observers below can read the materials and animation clips. A
/// load failure (usually a `.glb` that hasn't been converted yet) is permanent, so log it
/// once and stop.
fn spawn_scene_when_ready(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    args: Res<Args>,
    scene_gltf: Res<SceneGltf>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    if let LoadState::Failed(err) = asset_server.load_state(&scene_gltf.0) {
        *done = true;
        error!(
            "zero_day: failed to load `{}` ({err}). Convert it first with convert.py \
             (see the example README).",
            args.scene.glb()
        );
        return;
    }
    if !asset_server.is_loaded_with_dependencies(&scene_gltf.0) {
        return;
    }
    *done = true;
    let mut scene = commands.spawn(WorldAssetRoot(
        asset_server.load(format!("{}#Scene0", args.scene.glb())),
    ));
    scene
        .observe(proc_scene)
        .observe(setup_flythrough_camera)
        .observe(start_animation);
    if !args.no_solari {
        // Nothing reads `RaytracingMesh3d` without `SolariPlugins`.
        scene.observe(setup_raytracing_meshes);
    }
}

// Scene processing when the scene loads

/// Makes the emissive panels into bright Solari light sources. This system multiplies
/// the emissive of each unique material once (instances share material handles, so a
/// per-entity multiply would compound), then gives each emissive instance its own
/// material clone. Solari reads the emissive from the material asset, so panels that
/// share a handle can only pulse together, and the clones (only ~230 instances) let
/// `animate_emissive` flare each panel independently. `--no-pulse` skips the clones.
fn proc_scene(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    has_std_mat: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    children: Query<&Children>,
    args: Res<Args>,
) {
    let mut processed: HashSet<AssetId<StandardMaterial>> = HashSet::new();
    let mut emissive_bases: HashMap<AssetId<StandardMaterial>, LinearRgba> = HashMap::new();
    for entity in children.iter_descendants(scene_ready.entity) {
        let Ok(mat_h) = has_std_mat.get(entity) else {
            continue;
        };

        if processed.insert(mat_h.id())
            && let Some(mut mat) = materials.get_mut(mat_h)
            && mat.emissive != LinearRgba::BLACK
        {
            mat.emissive *= args.emissive;
            emissive_bases.insert(mat_h.id(), mat.emissive);
        }

        let Some(base) = emissive_bases.get(&mat_h.id()).copied() else {
            continue;
        };
        if args.no_pulse {
            continue;
        }
        let Some(material) = materials.get(mat_h.id()).cloned() else {
            continue;
        };
        let handle = materials.add(material);
        // A stable phase for each panel, from the low bits of the entity ID.
        let phase = ((entity.to_bits() & 0xffff) as f32 * PULSE_PHASE_STRIDE) % TAU;
        commands
            .entity(entity)
            .insert((MeshMaterial3d(handle), EmissivePanel { base, phase }));
    }
}

/// Strips the render components from the imported film camera and copies its field of
/// view and near plane to the render camera. The film camera keeps its animated
/// transform, which `drive_flythrough` follows.
///
/// The far plane isn't copied. The film's far=100 would cut off the ~700-unit shaft in
/// measure_seven, so the render camera keeps its far=2000. The near plane (0.001) must be
/// copied, because Solari gets primary visibility from a rasterized prepass and the
/// flythrough passes within ~0.02 units of geometry, which the default near plane of 0.1
/// would clip away. Bevy's reverse-z depth buffer keeps enough precision between 0.001
/// and 2000.
///
/// `FilmCamera` goes on the first camera only, so the `single()` call in
/// `drive_flythrough` can never find more than one.
#[allow(clippy::type_complexity)]
fn setup_flythrough_camera(
    scene_ready: On<WorldInstanceReady>,
    children: Query<&Children>,
    film_cameras: Query<(Entity, &Projection), (With<Camera>, Without<RenderCamera>)>,
    mut render_projection: Query<&mut Projection, With<RenderCamera>>,
    mut commands: Commands,
) {
    let mut film_projection = None;
    for entity in children.iter_descendants(scene_ready.entity) {
        let Ok((camera, projection)) = film_cameras.get(entity) else {
            continue;
        };
        let mut camera = commands.entity(camera);
        camera.remove::<(Camera3d, Camera, Projection)>();
        if film_projection.is_none() {
            camera.insert(FilmCamera);
            film_projection = Some(projection.clone());
        }
    }
    let Some(Projection::Perspective(film)) = film_projection else {
        return;
    };
    let Ok(mut render_projection) = render_projection.single_mut() else {
        return;
    };
    let Projection::Perspective(p) = &mut *render_projection else {
        return;
    };
    p.fov = film.fov;
    p.near = film.near;
}

/// Adds `RaytracingMesh3d` to each mesh and widens 16-bit indices to 32-bit, which the
/// Solari BLAS build needs. `convert.py` prepares the rest of the mesh layout, but it
/// can't set the index width because the glTF exporter always writes the smallest
/// sufficient index type.
fn setup_raytracing_meshes(
    scene_ready: On<WorldInstanceReady>,
    children: Query<&Children>,
    mesh_query: Query<&Mesh3d>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    for descendant in children.iter_descendants(scene_ready.entity) {
        let Ok(Mesh3d(mesh_handle)) = mesh_query.get(descendant) else {
            continue;
        };
        commands
            .entity(descendant)
            .insert(RaytracingMesh3d(mesh_handle.clone()));

        // Check with `get`, since `get_mut` would mark the mesh Modified and trigger a
        // re-upload and BLAS rebuild.
        let needs_widening = matches!(
            meshes.get(mesh_handle).and_then(Mesh::indices),
            Some(Indices::U16(_))
        );
        if needs_widening
            && let Some(mut mesh) = meshes.get_mut(mesh_handle)
            && let Some(indices) = mesh.indices_mut()
        {
            *indices = Indices::U32(indices.iter().map(|i| i as u32).collect());
        }
    }
}

// Runtime

/// Plays the imported animation (approximately 550 objects plus the film camera) in a
/// loop. The Blender exporter writes one clip per object (thousands per scene), all on
/// the film's shared timeline.
#[allow(clippy::too_many_arguments)]
fn start_animation(
    scene_ready: On<WorldInstanceReady>,
    scene_gltf: Res<SceneGltf>,
    gltfs: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut film_length: ResMut<FilmLength>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
    mut commands: Commands,
) {
    let Some(gltf) = gltfs.get(&scene_gltf.0) else {
        warn!("zero_day: glTF asset not ready; animations won't play");
        return;
    };

    // All clips share one timeline, so the longest clip gives the film length.
    film_length.0 = gltf
        .animations
        .iter()
        .filter_map(|h| clips.get(h).map(AnimationClip::duration))
        .fold(0.0_f32, f32::max);

    let (graph, nodes) = AnimationGraph::from_clips(gltf.animations.iter().cloned());
    let graph = graphs.add(graph);
    for entity in children.iter_descendants(scene_ready.entity) {
        if let Ok(mut player) = players.get_mut(entity) {
            for node in &nodes {
                player.play(*node).repeat();
            }
            commands
                .entity(entity)
                .insert(AnimationGraphHandle(graph.clone()));
        }
    }
    info!(
        "zero_day: started {} animation clip(s) ({:.1}s take)",
        nodes.len(),
        film_length.0
    );
}

/// Changes between the film flythrough and free-fly.
fn toggle_flythrough(input: Res<ButtonInput<KeyCode>>, mut cinematic: ResMut<Cinematic>) {
    if input.just_pressed(KeyCode::KeyC) {
        cinematic.active = !cinematic.active;
    }
}

/// The DLSS Ray Reconstruction component, at the quality that `--dlss-quality` selects.
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
fn dlss_rr(perf_quality_mode: DlssPerfQualityMode) -> Dlss<DlssRayReconstructionFeature> {
    Dlss::<DlssRayReconstructionFeature> {
        perf_quality_mode,
        reset: Default::default(),
        _phantom_data: Default::default(),
    }
}

/// Turns DLSS Ray Reconstruction on and off with the `N` key.
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
fn toggle_denoiser(
    input: Res<ButtonInput<KeyCode>>,
    args: Res<Args>,
    camera: Single<(Entity, Has<Dlss<DlssRayReconstructionFeature>>), With<RenderCamera>>,
    dlss_rr_supported: Option<Res<DlssRayReconstructionSupported>>,
    mut commands: Commands,
) {
    if !input.just_pressed(KeyCode::KeyN) || dlss_rr_supported.is_none() {
        return;
    }
    let (entity, has_dlss) = *camera;
    if has_dlss {
        // DLSS inserted `TemporalJitter` and `MipBias`; remove them with it.
        commands
            .entity(entity)
            .remove::<(Dlss<DlssRayReconstructionFeature>, TemporalJitter, MipBias)>();
    } else {
        commands.entity(entity).insert(dlss_rr(args.dlss_quality));
    }
}

/// The mean of the lowest 1% and the mean of the highest 1% of `samples`. Sorts
/// `samples` in place.
fn one_percent_extremes(samples: &mut [f64]) -> (f64, f64) {
    samples.sort_by(f64::total_cmp);
    let count = (samples.len() / 100).max(1);
    let mean = |s: &[f64]| s.iter().sum::<f64>() / count as f64;
    (
        mean(&samples[..count]),
        mean(&samples[samples.len() - count..]),
    )
}

/// Calculates the average frame time and the slowest 1% frame time from the diagnostics
/// history, for the HUD.
fn frame_stats(
    diagnostics: Res<DiagnosticsStore>,
    mut stats: ResMut<FrameStats>,
    mut scratch: Local<Vec<f64>>,
) {
    let Some(frame_time) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME) else {
        return;
    };
    stats.avg_ms = frame_time.average().unwrap_or_default();
    if frame_time.history_len() >= 100 {
        scratch.clear();
        scratch.extend(frame_time.measurements().map(|m| m.value));
        stats.one_percent_high_ms = one_percent_extremes(&mut scratch).1;
    }
}

/// The `B` key rewinds the flythrough, measures one full loop, and prints a summary.
/// Rewinding and switching to cinematic mode make the runs comparable. The percentiles
/// are calculated once at the end, so the 1% values cover the whole run.
#[allow(clippy::too_many_arguments)]
fn benchmark(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    film_length: Res<FilmLength>,
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    mesh_instances: Query<&Mesh3d>,
    mut players: Query<&mut AnimationPlayer>,
    mut cinematic: ResMut<Cinematic>,
    mut running: Local<Option<Instant>>,
    mut samples: Local<Vec<f64>>,
) {
    if input.just_pressed(KeyCode::KeyB) && running.is_none() && film_length.0 > 0.0 {
        cinematic.active = true;
        for mut player in &mut players {
            player.rewind_all();
        }
        *running = Some(Instant::now());
        samples.clear();
        println!(
            "zero_day: benchmarking one flythrough loop (~{:.1}s)...",
            film_length.0
        );
    }
    let Some(start) = *running else {
        return;
    };
    samples.push(time.delta_secs_f64() * 1000.0);
    let elapsed = start.elapsed().as_secs_f64();
    if elapsed < film_length.0 as f64 {
        return;
    }

    let frames = samples.len();
    println!(
        "  {:.2} ms/frame avg  ({:.0} fps)  over {frames} frames",
        elapsed * 1000.0 / frames as f64,
        frames as f64 / elapsed,
    );
    let (low, high) = one_percent_extremes(&mut samples);
    println!("  1% low {low:.2} ms | 1% high {high:.2} ms");
    println!(
        "  {} meshes | {} instances | {} materials",
        meshes.len(),
        mesh_instances.iter().count(),
        materials.len()
    );
    *running = None;
}

/// Moves the render camera with the animated transform of the film camera while
/// cinematic mode is active. This is the original Zero-Day flythrough.
fn drive_flythrough(
    cinematic: Res<Cinematic>,
    film: Query<&GlobalTransform, With<FilmCamera>>,
    mut render: Query<&mut Transform, With<RenderCamera>>,
) {
    if !cinematic.active {
        return;
    }
    let Ok(film) = film.single() else {
        return;
    };
    let Ok(mut render) = render.single_mut() else {
        return;
    };
    // Copy only the position and orientation, because the film camera's global transform
    // can carry a scale from its parent.
    let film = film.compute_transform();
    render.translation = film.translation;
    render.rotation = film.rotation;
}

/// A substitute for the film's animated lights, which aren't in the asset. A wave of
/// brightness moves along the corridor, sharpened into distinct flares, with a stable
/// per-panel phase so adjacent panels don't pulse together. Each panel is its own Solari
/// area light, so each flare really lights the corridor as it moves.
fn animate_emissive(
    time: Res<Time>,
    panels: Query<(
        &GlobalTransform,
        &EmissivePanel,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let t = time.elapsed_secs();
    for (transform, panel, material) in &panels {
        let z = transform.translation().z;
        let wave = ops::sin(t * PULSE_FREQ - z * PULSE_WAVE_NUMBER + panel.phase);
        // Sharpen the sine so each panel stays dim and flares quickly.
        let flare = ops::powf(0.5 + 0.5 * wave, PULSE_SHARPNESS);
        let level = PULSE_FLOOR + (PULSE_PEAK - PULSE_FLOOR) * flare;
        if let Some(mut mat) = materials.get_mut(material.id()) {
            mat.emissive = panel.base * level;
        }
    }
}

fn update_hud(
    stats: Res<FrameStats>,
    diagnostics: Res<DiagnosticsStore>,
    cinematic: Res<Cinematic>,
    args: Res<Args>,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] denoiser: Single<
        Has<Dlss<DlssRayReconstructionFeature>>,
        With<RenderCamera>,
    >,
    mut text: Single<&mut Text, With<HudText>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or_default();
    let mode = if cinematic.active {
        "flythrough  (C: free-fly)"
    } else {
        "free-fly  (C: flythrough)"
    };

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let dlss_line = match (args.no_solari, *denoiser) {
        (true, _) => "",
        (false, true) => "\nDLSS-RR: on  (N)",
        (false, false) => "\nDLSS-RR: off  (N)",
    };
    #[cfg(not(all(feature = "dlss", not(feature = "force_disable_dlss"))))]
    let dlss_line = "";

    let title = if args.no_solari {
        "Zero-Day (NO SOLARI: not representative)"
    } else {
        "Zero-Day (Solari)"
    };
    text.0 = format!(
        "{title}\n{fps:>5.0} fps | {:.1} ms avg | {:.1} ms 1%-worst\n{mode}\nB: benchmark{dlss_line}",
        stats.avg_ms, stats.one_percent_high_ms,
    );
}
