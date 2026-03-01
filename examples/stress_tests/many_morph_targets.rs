//! Simple benchmark to test rendering many meshes with animated morph targets.

use argh::FromArgs;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    post_process::motion_blur::MotionBlur,
    prelude::*,
    scene::SceneInstanceReady,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use chacha20::ChaCha8Rng;
use core::{f32::consts::PI, str::FromStr};
use rand::{RngExt, SeedableRng};

/// Controls the morph weights.
#[derive(PartialEq)]
enum ArgWeights {
    /// Weights will be animated by an `AnimationClip`.
    Animated,

    /// Set all the weights to one.
    One,

    /// Set all the weights to zero, minimizing vertex shader cost.
    Zero,

    /// Set all the weights to a very small value, so the pixel shader cost
    /// should be similar to `Zero` but vertex shader cost the same as `One`.
    Tiny,
}

impl FromStr for ArgWeights {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "animated" => Ok(Self::Animated),
            "zero" => Ok(Self::Zero),
            "one" => Ok(Self::One),
            "tiny" => Ok(Self::Tiny),
            _ => Err("must be 'animated', 'one', `zero`, or 'tiny'".into()),
        }
    }
}

/// Controls the camera.
#[derive(PartialEq)]
enum ArgCamera {
    /// Fill the screen with meshes.
    Near,

    /// Zoom far out. This is used to reduce pixel shader costs and so emphasize
    /// vertex shader costs.
    Far,
}

impl FromStr for ArgCamera {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "near" => Ok(Self::Near),
            "far" => Ok(Self::Far),
            _ => Err("must be 'near' or 'far'".into()),
        }
    }
}

/// Controls how the meshes spawn.
#[derive(PartialEq)]
enum ArgSpawning {
    /// All meshes will spawn in one frame.
    Instant,

    /// One mesh will spawn per frame.
    Gradual,

    /// Spawn one mesh per frame in a consistent order until all are spawned,
    /// then despawn one mesh per frame in the same order, and repeat.
    RegularCycle,

    /// Spawn one mesh per frame in a random order until all are spawned, then
    /// despawn one mesh per frame in a random order, and repeat.
    RandomCycle,

    /// All meshes will spawn in one frame, and after that one mesh will spawn
    /// and one mesh will despawn per frame.
    RandomSteady,
}

impl FromStr for ArgSpawning {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "instant" => Ok(Self::Instant),
            "gradual" => Ok(Self::Gradual),
            "regular-cycle" => Ok(Self::RegularCycle),
            "random-cycle" => Ok(Self::RandomCycle),
            "random-steady" => Ok(Self::RandomSteady),
            _ => Err(
                "must be 'instant', 'gradual', 'regular-cycle', 'random-cycle', or 'random-steady'"
                    .into(),
            ),
        }
    }
}

/// `many_morph_targets` stress test
#[derive(FromArgs, Resource)]
struct Args {
    /// number of meshes - default = 1024
    #[argh(option, default = "1024")]
    count: usize,

    /// options: 'animated', 'one', 'zero', 'tiny' - default = 'animated'
    #[argh(option, default = "ArgWeights::Animated")]
    weights: ArgWeights,

    /// options: 'near', 'far' - default = 'near'
    #[argh(option, default = "ArgCamera::Near")]
    camera: ArgCamera,

    /// options: 'instant', 'gradual', 'regular-cycle', 'random-cycle', 'random-steady' - default = 'instant'
    #[argh(option, default = "ArgSpawning::Instant")]
    spawning: ArgSpawning,

    /// enable motion blur
    #[argh(switch)]
    motion_blur: bool,
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
                    title: "Many Morph Targets".to_string(),
                    present_mode: PresentMode::AutoNoVsync,
                    resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .insert_resource(WinitSettings::continuous())
        .insert_resource(GlobalAmbientLight {
            brightness: 1000.0,
            ..Default::default()
        })
        .insert_resource(MorphAssets::default())
        .insert_resource(Rng(ChaCha8Rng::seed_from_u64(856673)))
        .insert_resource(State::new(&args))
        .insert_resource(args)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

#[derive(Resource, Default)]
struct MorphAssets {
    scene: Handle<Scene>,
    animations: Vec<(Handle<AnimationGraph>, AnimationNodeIndex)>,
}

#[derive(Component, Clone)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
    speed: f32,
}

fn dims(count: usize) -> (usize, usize) {
    let x_dim = ((count as f32).sqrt().ceil() as usize).max(1);
    let y_dim = count.div_ceil(x_dim);

    (x_dim, y_dim)
}

fn setup(
    args: Res<Args>,
    mut commands: Commands,
    mut assets: ResMut<MorphAssets>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    state: Res<State>,
) {
    let (x_dim, _) = dims(state.slot_count);

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_rotation_z(PI / 2.0)),
    ));

    let camera_distance = (x_dim as f32)
        * match args.camera {
            ArgCamera::Near => 4.0,
            ArgCamera::Far => 200.0,
        };

    let mut camera = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, camera_distance).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    if args.motion_blur {
        camera.insert((
            MotionBlur {
                // Use an unrealistically large shutter angle so that motion blur is clearly visible.
                shutter_angle: 3.0,
                ..Default::default()
            },
            // MSAA and MotionBlur are not compatible on WebGL.
            #[cfg(all(feature = "webgl2", target_arch = "wasm32", not(feature = "webgpu")))]
            Msaa::Off,
        ));
    }

    const ASSET_PATH: &str = "models/animated/MorphStressTest.gltf";

    *assets = MorphAssets {
        scene: asset_server.load(GltfAssetLabel::Scene(0).from_asset(ASSET_PATH)),
        animations: (0..3)
            .map(|gltf_index| {
                let (graph, index) = AnimationGraph::from_clip(
                    asset_server.load(GltfAssetLabel::Animation(gltf_index).from_asset(ASSET_PATH)),
                );
                (graphs.add(graph), index)
            })
            .collect::<Vec<_>>(),
    }
}

enum CycleState {
    Spawn,
    Despawn,
}

#[derive(Resource)]
struct State {
    ticks: usize,
    slot_count: usize,
    spawned: Vec<(usize, Entity)>,
    despawned: Vec<usize>,
    cycle: CycleState,
}

impl State {
    fn new(args: &Args) -> State {
        // The `RandomSteady` case allocates double the number of slots but only
        // keeps half occupied.
        let slot_count = match args.spawning {
            ArgSpawning::RandomSteady => args.count * 2,
            _ => args.count,
        };

        State {
            ticks: 0,
            slot_count,
            spawned: Default::default(),
            despawned: (0..slot_count).collect::<Vec<_>>(),
            cycle: CycleState::Spawn,
        }
    }
}

#[derive(Resource)]
struct Rng(ChaCha8Rng);

// Randomly take `count` entries from the given `Vec` and return them.
fn take_random<T>(rng: &mut ChaCha8Rng, from: &mut Vec<T>, count: usize) -> Vec<T> {
    (0..count)
        .map(|_| from.swap_remove(rng.random_range(..from.len())))
        .collect()
}

fn update(
    args: Res<Args>,
    mut commands: Commands,
    mut state: ResMut<State>,
    mut rng: ResMut<Rng>,
    assets: Res<MorphAssets>,
) {
    state.ticks += 1;

    if state.spawned.is_empty() {
        state.cycle = CycleState::Spawn;
    } else if state.despawned.is_empty() {
        state.cycle = CycleState::Despawn;
    }

    let mut to_spawn = Vec::<usize>::default();
    let mut to_despawn = Vec::<(usize, Entity)>::default();

    match args.spawning {
        ArgSpawning::Instant => to_spawn = std::mem::take(&mut state.despawned),
        ArgSpawning::Gradual => to_spawn = state.despawned.pop().into_iter().collect(),
        ArgSpawning::RegularCycle => match state.cycle {
            CycleState::Spawn => to_spawn.push(state.despawned.pop().unwrap()),
            CycleState::Despawn => to_despawn.push(state.spawned.pop().unwrap()),
        },
        ArgSpawning::RandomCycle => match state.cycle {
            CycleState::Spawn => to_spawn = take_random(&mut rng.0, &mut state.despawned, 1),
            CycleState::Despawn => to_despawn = take_random(&mut rng.0, &mut state.spawned, 1),
        },
        ArgSpawning::RandomSteady => {
            if state.spawned.is_empty() {
                let spawn_count = state.slot_count / 2;
                to_spawn = take_random(&mut rng.0, &mut state.despawned, spawn_count);
            } else {
                to_spawn = take_random(&mut rng.0, &mut state.despawned, 1);
                to_despawn = take_random(&mut rng.0, &mut state.spawned, 1);
            }
        }
    }

    for (mesh_index, entity) in to_despawn {
        commands.entity(entity).despawn();
        state.despawned.push(mesh_index);
    }

    for mesh_index in to_spawn {
        // Arrange the meshes in a grid.

        let (x_dim, y_dim) = dims(state.slot_count);

        let x = 2.5 + (5.0 * ((mesh_index.rem_euclid(x_dim) as f32) - ((x_dim as f32) * 0.5)));
        let y = -2.2 - (3.0 * ((mesh_index.div_euclid(x_dim) as f32) - ((y_dim as f32) * 0.5)));

        // Vary the animation speed so that the number of morph targets
        // active on each frame is more likely to be stable.

        let speed = ((mesh_index as f32) * 0.1).rem_euclid(1.0) + 0.5;

        let animation_asset =
            assets.animations[mesh_index.rem_euclid(assets.animations.len())].clone();
        let animation = AnimationToPlay {
            graph_handle: animation_asset.0.clone(),
            index: animation_asset.1,
            speed,
        };

        let entity = commands
            .spawn((
                animation,
                Transform::from_xyz(x, y, 0.0),
                SceneRoot(assets.scene.clone()),
            ))
            .observe(play_animation)
            .observe(set_weights)
            .id();

        state.spawned.push((mesh_index, entity));
    }
}

fn play_animation(
    trigger: On<SceneInstanceReady>,
    mut commands: Commands,
    args: Res<Args>,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if args.weights == ArgWeights::Animated
        && let Ok(animation_to_play) = animations_to_play.get(trigger.entity)
    {
        for child in children.iter_descendants(trigger.entity) {
            if let Ok(mut player) = players.get_mut(child) {
                commands
                    .entity(child)
                    .insert(AnimationGraphHandle(animation_to_play.graph_handle.clone()));

                player
                    .play(animation_to_play.index)
                    .repeat()
                    .set_speed(animation_to_play.speed);
            }
        }
    }
}

fn set_weights(
    trigger: On<SceneInstanceReady>,
    args: Res<Args>,
    children: Query<&Children>,
    mut weight_components: Query<&mut MorphWeights>,
) {
    if let Some(weight_value) = match args.weights {
        ArgWeights::One => Some(1.0),
        ArgWeights::Zero => Some(0.0),
        ArgWeights::Tiny => Some(0.00001),
        _ => None,
    } {
        for child in children.iter_descendants(trigger.entity) {
            if let Ok(mut weight_component) = weight_components.get_mut(child) {
                weight_component.weights_mut().fill(weight_value);
            }
        }
    }
}
