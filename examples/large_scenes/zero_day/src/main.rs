//! Beeple's "Zero-Day" sci-fi corridor (NVIDIA ORCA), path-traced with Bevy Solari.
//!
//! Zero-Day has no punctual lights. All of its light comes from approximately 10,000
//! emissive triangles, as in NVIDIA's original real-time
//! ["Measure 1"](https://www.youtube.com/watch?v=0WE7CgJMuVc) demo. This example needs
//! Bevy Solari, because only a path tracer can light the scene this way. Solari makes the
//! emissive meshes into area lights that give global illumination.
//!
//! The example plays the animation of the film: approximately 550 objects and the camera
//! flythrough. The render camera follows the film camera.
//!
//! No ORCA measure contains animated *lights*. Octane made the emissive pulses of the
//! film procedurally. They are in no exported asset, and glTF cannot carry them, because
//! Bevy does not support `KHR_animation_pointer`. `animate_emissive` makes a substitute:
//! a wave of light that moves along the emissive panels of the corridor.
//!
//! `--scene` selects the ORCA measure: `measure_one` (the default), `measure_seven`, or
//! `measure_seven_colored_lights`. `convert.py` makes a different `.glb` for each
//! measure. The measures have different geometry and different emissive colors. No
//! measure has animated lights.
//!
//! Controls: `C` changes between the film flythrough and free-fly (WASD and mouse), `N`
//! turns DLSS Ray Reconstruction on and off, `B` does a short benchmark and prints the
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

// DLSS Ray Reconstruction removes the noise from the Solari output when the `dlss`
// feature is on. It needs an NVIDIA RTX GPU. Without one, the path tracer still runs.
// DLSS adds `TemporalJitter` and `MipBias`, and the `N` key removes them together with
// the denoiser.
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
    /// which ORCA measure to load: measure_one (default), measure_seven, or
    /// measure_seven_colored_lights. `convert.py` makes a different `.glb` for each.
    #[argh(option, default = "Scene::MeasureOne")]
    scene: Scene,

    /// emissive multiplier for the accent panels (default 150000). The panels are the only
    /// lights in the scene, and they must be bright to light the space.
    #[argh(option, default = "DEFAULT_EMISSIVE")]
    emissive: f32,

    /// disable the synthetic emissive pulse. By default, a wave of light moves along the
    /// panels as a substitute for the animated lights of the film, which are not in the
    /// exported asset.
    #[argh(switch)]
    no_pulse: bool,

    /// combine the thousands of per-object clips of the glTF into one clip at startup. The
    /// playback is identical. The load is slower, but the animation evaluation in each frame
    /// is much less expensive.
    #[argh(switch)]
    merge_animations: bool,

    /// skip Solari and use a flat ambient light. The scene does not render correctly this
    /// way, because the panels that Solari resolves are its only real lights. This is an
    /// escape hatch for profiling and smoke tests, not a lighting mode. It runs on GPUs
    /// that cannot do ray tracing.
    #[argh(switch)]
    no_solari: bool,

    /// render resolution as `WxH` (default 1920x1080). The Solari cost increases with the
    /// pixel count. Use a lower value (for example `1280x720`) on the heavy measures to
    /// get more frames per second, with less sharpness.
    #[argh(option, default = "(1920, 1080)", from_str_fn(parse_resolution))]
    resolution: (u32, u32),

    /// DLSS quality mode: auto (default), dlaa, quality, balanced, performance, or
    /// ultra_performance. A lower mode renders at a smaller internal resolution and gives
    /// more frames per second.
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

/// The ORCA "Zero-Day" measure to load. Each measure converts to its own self-contained
/// `.glb` (see `convert.py` and the README). The flythrough camera and the emissive
/// handling are the same for all three measures. Only the asset filename changes.
// Keep the shared `Measure` prefix: it is the same as the ORCA asset names.
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy)]
enum Scene {
    MeasureOne,
    MeasureSeven,
    MeasureSevenColoredLights,
}

impl Scene {
    /// The `.glb` file that this measure loads from `assets/`. Git ignores that folder.
    fn glb(self) -> &'static str {
        match self {
            Scene::MeasureOne => "zero_day_measure_one.glb",
            Scene::MeasureSeven => "zero_day_measure_seven.glb",
            Scene::MeasureSevenColoredLights => "zero_day_measure_seven_colored_lights.glb",
        }
    }
}

/// Default emissive multiplier. `--emissive` replaces it. This is approximately the
/// luminance of a bright LED fixture in nits. This one value is correct for all three
/// measures.
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

// Tuning values for the synthetic pulse (see `animate_emissive`). The film sequences its
// lights procedurally. These values make a substitute: a wave that moves along the
// corridor. You can change all of them.
/// Rate of the wave in time (rad/s).
const PULSE_FREQ: f32 = 2.0;
/// Spatial frequency along the Z axis of the corridor (rad/world-unit). This sets the
/// wavelength. Panels at different depths then flare at different times.
const PULSE_WAVE_NUMBER: f32 = 0.05;
/// Exponent that makes the sine sharper and gives distinct flares. A higher value makes
/// each flare more sudden.
const PULSE_SHARPNESS: f32 = 2.0;
/// Minimum and maximum levels, as a fraction of the base emissive of each panel. The
/// minimum keeps the corridor lit between the flares. The maximum is more than 1.0, and
/// the panels bloom when they flare.
const PULSE_FLOOR: f32 = 0.4;
const PULSE_PEAK: f32 = 1.8;
/// Golden angle (rad). This gives each panel a different but stable phase. Panels at the
/// same depth then do not flare together.
const PULSE_PHASE_STRIDE: f32 = 2.399_963_2;

fn main() {
    let args: Args = argh::from_env();
    let (win_w, win_h) = args.resolution;

    let mut app = App::new();

    // Set the DLSS project ID before the plugins: DLSS reads the ID when the renderer
    // starts.
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    app.insert_resource(DlssProjectId(bevy::asset::uuid::uuid!(
        "b1a7c0de-4d2f-4e6a-9b3c-0d1e2f3a4b5c"
    )));

    let no_solari = args.no_solari;

    app.insert_resource(ClearColor(Color::BLACK))
        // Solari makes all of the light from the emissive meshes. The scene has no
        // ambient fill. With `--no-solari`, nothing makes light from those meshes. A
        // flat ambient light then keeps the geometry visible. It is a substitute, not the
        // lighting of the scene.
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
                // The HUD sorts the frame-time history and makes its text again, which
                // makes the UI layout dirty. At 10 Hz that cost stays out of the frame
                // times this example measures, and the numbers are also easier to read.
                (frame_stats, update_hud)
                    .chain()
                    .run_if(on_timer(Duration::from_millis(100))),
            ),
        );

    // The plugin, not the camera `SolariLighting` component, requests the ray-tracing
    // device features. Thus `--no-solari` must skip the plugin to run on any GPU.
    if !no_solari {
        app.add_plugins(SolariPlugins);
    }

    // The `N` key turns DLSS on and off. Ray Reconstruction removes the noise from the
    // Solari output. It has no function when Solari is off.
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    app.add_systems(Update, toggle_denoiser.run_if(move || !no_solari));

    app.run();
}

// Components and resources

/// The render camera of the example. It holds the HDR and Solari components.
#[derive(Component)]
struct RenderCamera;

/// The imported film camera. Each measure gives it a different name (`DynamicCamera2`,
/// `DynamicCamera`, and others). Only its transform stays, and `drive_flythrough` follows
/// that transform.
#[derive(Component)]
struct FilmCamera;

/// The readout on the screen.
#[derive(Component)]
struct HudText;

/// Keeps the full glTF loaded. Its animation clips stay available while it is loaded.
#[derive(Resource)]
struct SceneGltf(Handle<Gltf>);

/// One emissive panel instance. The panels share a small number of materials, and
/// `proc_scene` gives each emissive instance its own material clone. `animate_emissive`
/// can then set the emissive of each panel independently: from the world position of the
/// panel, for a wave along the corridor, and from a stable `phase`, so that adjacent
/// panels do not flare together.
#[derive(Component)]
struct EmissivePanel {
    /// The increased base emissive, as `proc_scene` calculated it. The pulse multiplies
    /// this value.
    base: LinearRgba,
    /// The stable phase offset of this panel (radians).
    phase: f32,
}

/// The length of the loaded film animation (seconds), from the longest animation clip.
/// `start_animation` sets it. The `B` benchmark uses it to measure exactly one loop of
/// the animation. Each measure has a different number of frames.
#[derive(Resource, Default)]
struct FilmLength(f32);

/// Frame-time statistics for the HUD. Solari is expensive, and these numbers show its
/// cost. `one_percent_high_ms` is the mean of the slowest 1% of the frames.
#[derive(Resource, Default)]
struct FrameStats {
    avg_ms: f64,
    one_percent_high_ms: f64,
}

/// Selects the camera mode: follow the animated film camera (the mode at startup), or
/// free-fly. The `C` key changes the mode.
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

    // Load the full glTF, not only the scene. `spawn_scene_when_ready` spawns the scene
    // when the load is complete. The full `Gltf` also keeps its animation clips available
    // to `start_animation`.
    commands.insert_resource(SceneGltf(asset_server.load(glb)));

    // The camera. `setup_flythrough_camera` replaces its field of view and its near plane
    // with the values of the film camera, but keeps the far plane. The transform below is
    // the view that the example shows until the flythrough starts.
    let mut cam = commands.spawn((
        Camera3d::default(),
        // The imported film camera also spawns as an active camera with order 0. A
        // higher order keeps this camera in control until `setup_flythrough_camera`
        // removes the other one. The rest pose of the film camera is never visible.
        Camera {
            order: 1,
            ..default()
        },
        Hdr,
        // Solari and DLSS need MSAA off. It also stays off with `--no-solari`, to keep
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
        // Glare on the panels. Their values are much higher than white.
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
    // DLSS Ray Reconstruction removes the noise from the path-traced output if the GPU
    // supports it. It reads the G-buffer outputs of Solari, and it stays off with
    // `--no-solari`.
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

/// Spawns the scene when its glTF and all of its dependencies (materials, meshes,
/// animation clips) are loaded, then does no more work. The `WorldInstanceReady` observers
/// read those sub-assets directly. The scene must not spawn before all of them are
/// available: without the materials, `proc_scene` cannot increase the emissive and the
/// corridor stays black, because the emissive panels are its only light; without the
/// clips, nothing moves. A load failure, usually a `.glb` that is not yet converted, is
/// permanent. This system logs it one time and then stops.
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
        // Increases the emissive of the materials and clones one material per emissive
        // instance.
        .observe(proc_scene)
        // Makes the film camera into the source of the flythrough transform.
        .observe(setup_flythrough_camera)
        // Plays the imported animation.
        .observe(start_animation);
    if !args.no_solari {
        // Adds `RaytracingMesh3d` to each mesh, which lets Solari trace against it.
        // Without `SolariPlugins`, nothing reads that component, and this observer is
        // not necessary.
        scene.observe(setup_raytracing_meshes);
    }
}

// Scene processing when the scene loads

/// Prepares the loaded scene. This system increases the emissive of the materials, which
/// makes the panels into bright Solari light sources. It then gives each emissive
/// *instance* its own material, which lets the pulse animate the instances independently.
/// `convert.py` does the material *repairs*: the normal-map convention, the alpha mode,
/// and the black emissive factors. It also exports no lights at all. Only the tuning for
/// this example stays here.
///
/// This system increases the emissive of each unique material one time. Instanced meshes
/// share a material handle. An increase for each entity would multiply a shared emissive
/// many times. `emissive_bases` holds the new value for all of the instances that use
/// that material.
///
/// Only approximately 230 instances are emissive, and one material clone for each
/// instance is not expensive. Solari reads the emissive from the material asset. Only
/// different clones can therefore make adjacent panels flare at different times, because
/// panels with one shared handle can only pulse together. With `--no-pulse`, nothing
/// animates the panels, and this system does not make the clones.
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

/// Makes the imported film camera into the source of the flythrough transform. This
/// system removes the render components from the film camera, and copies its field of
/// view and its near plane to the render camera. Only the render camera then draws. The
/// film camera keeps its animated transform, which `drive_flythrough` follows. The camera
/// entity is available at `WorldInstanceReady`.
///
/// This system copies the field of view and the near plane, but not the far plane. The
/// film has near=0.001 and far=100. The render camera keeps its far=2000, because the
/// measures continue much further than 100 units from the camera. The shaft in
/// measure_seven is approximately 700 units deep, and far=100 would cut it short.
///
/// The small near plane is necessary, and this system must copy it. Solari gets primary
/// visibility from a *rasterized* depth and G-buffer prepass, which clips all geometry
/// nearer than the near plane. The flythrough moves within approximately 0.02 units of the
/// geometry, because measure seven goes through dense machinery. With a near plane at 0.1,
/// the prepass clips those surfaces to empty depth, and the camera appears to look through
/// them. The value of 0.001 in the film is correct for that geometry, and the reverse-z
/// depth buffer of Bevy keeps sufficient precision between 0.001 and 2000.
///
/// Each measure has one camera only. This system nevertheless adds `FilmCamera` to the
/// first camera only, and removes the render components from all of them. The `single()`
/// call in `drive_flythrough` can thus never find more than one camera.
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

/// Lets Solari trace against the meshes of the scene. This system adds `RaytracingMesh3d`
/// to each mesh and changes 16-bit indices to 32-bit indices, which the BLAS build of
/// Solari needs.
///
/// `convert.py` makes the vertex layout that Solari also needs: POSITION, NORMAL, UV0, and
/// TANGENT only. `convert.py` owns the asset and does this one time, not at each run. It
/// cannot set the index width, because the glTF exporter has no 32-bit option. The `.glb`
/// contains the smallest index type that is sufficient.
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

        // Examine the mesh with `get`: a call to `get_mut` marks it as Modified, which
        // causes an upload and a BLAS build again.
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

/// Plays the imported animation on the animation player of the scene, in a loop. The
/// animation moves approximately 550 objects and the film camera.
///
/// `convert.py` exports with the glTF `SCENE` animation mode to get one baked clip, but
/// the Blender exporter still writes one clip *for each object*: 2315 clips for
/// measure_one and 5032 clips for measure_seven. In each frame, the `AnimationPlayer`
/// then advances that number of `ActiveAnimation`s and evaluates a graph of the same
/// width. This cost comes only from the number of clips. `--merge-animations` combines
/// them into one clip at startup. The curves of each object have a different
/// `AnimationTargetId` and thus never collide, and the playback is identical with one
/// active animation in the place of thousands. The clips contain no events, because FBX
/// animation is rigid TRS, and thus no events are lost.
///
/// This system runs on `WorldInstanceReady`, as in `animated_mesh.rs`. When the scene has
/// spawned, its parent glTF and the `animations` are loaded, and the player is one of its
/// descendants. The system is thus reliable and does not have to poll.
#[allow(clippy::too_many_arguments)]
fn start_animation(
    scene_ready: On<WorldInstanceReady>,
    scene_gltf: Res<SceneGltf>,
    gltfs: Res<Assets<Gltf>>,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut film_length: ResMut<FilmLength>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
    args: Res<Args>,
    mut commands: Commands,
) {
    let Some(gltf) = gltfs.get(&scene_gltf.0) else {
        warn!("zero_day: glTF asset not ready; animations will not play");
        return;
    };

    // The longest source clip gives the length of the animation, because the SCENE bake
    // puts all of the objects on one shared timeline. This length sets the benchmark
    // interval for each measure.
    film_length.0 = gltf
        .animations
        .iter()
        .filter_map(|h| clips.get(h).map(AnimationClip::duration))
        .fold(0.0_f32, f32::max);

    let (graph, nodes) = if args.merge_animations {
        let mut merged = AnimationClip::default();
        let mut source_clips = 0;
        for handle in &gltf.animations {
            // Use `remove`, not `get`: the curves move into the merged clip. A clone
            // would keep both copies in memory, because `SceneGltf` holds the handles of
            // the source clips.
            let Some(mut clip) = clips.remove(handle) else {
                continue;
            };
            for (target_id, curves) in clip.curves_mut().drain() {
                merged
                    .curves_mut()
                    .entry(target_id)
                    .or_default()
                    .extend(curves);
            }
            source_clips += 1;
        }
        merged.set_duration(film_length.0);
        info!("zero_day: merged {source_clips} clips into one");

        let (graph, node) = AnimationGraph::from_clip(clips.add(merged));
        (graph, vec![node])
    } else {
        AnimationGraph::from_clips(gltf.animations.iter().cloned())
    };

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

/// Turns DLSS Ray Reconstruction on and off with the `N` key, as the `solari` example
/// does. DLSS also owns `TemporalJitter` and `MipBias`, and this system removes them
/// together with DLSS.
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
        commands
            .entity(entity)
            .remove::<(Dlss<DlssRayReconstructionFeature>, TemporalJitter, MipBias)>();
    } else {
        commands.entity(entity).insert(dlss_rr(args.dlss_quality));
    }
}

/// The mean of the lowest 1% and the mean of the highest 1% of `samples`. This function
/// sorts `samples` in place. It always uses one sample or more from each end.
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

/// The `B` key starts the flythrough again from the first frame, measures one loop, and
/// prints a summary. The rewind and the change to cinematic mode make the runs comparable:
/// the camera path and the object movement are the same each time. This system does
/// nothing before the animation is loaded, because its length sets the measured interval.
///
/// This system records the frame times and calculates the percentiles one time at the end.
/// The 1% values thus apply to the full run, and not to the contents of `FrameStats` at
/// one moment.
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
        // Start the animation again from frame 0 and follow the film camera. The
        // measured interval is then identical for each run.
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
    // Copy the position and the orientation only. The global transform of the film camera
    // can contain a scale from its parent, which the render camera must not have.
    let film = film.compute_transform();
    render.translation = film.translation;
    render.rotation = film.rotation;
}

/// Makes a substitute for the animated lights of the film: a wave of brightness that
/// moves along the corridor. The wave is sharpened into distinct flares, and each panel
/// has a stable phase, and adjacent panels thus do not pulse together. With Solari, this
/// changes the real illumination, because each panel is its own area light and each flare
/// lights the corridor as it moves. Octane made the sequence in the film procedurally, and
/// it is not in the asset. This wave is only similar to it. With `--no-pulse`, the panels
/// keep their constant increased emissive.
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
        // A wave along the long (Z) axis of the corridor, with a different offset for
        // each panel. The full scene then shimmers and does not flash as one light.
        let z = transform.translation().z;
        let wave = ops::sin(t * PULSE_FREQ - z * PULSE_WAVE_NUMBER + panel.phase);
        // Sharpen the sine. Each panel then stays dim and flares quickly.
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
