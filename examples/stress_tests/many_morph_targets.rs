//! Simple benchmark to test rendering many meshes with animated morph targets.

use argh::FromArgs;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    scene::SceneInstanceReady,
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};
use core::{f32::consts::PI, str::FromStr};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(PartialEq)]
enum ArgWeights {
    Animated,
    All,
    None,
}

impl FromStr for ArgWeights {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "animated" => Ok(Self::Animated),
            "none" => Ok(Self::None),
            "all" => Ok(Self::All),
            _ => Err("must be 'animated', 'none', or 'all'".into()),
        }
    }
}

#[derive(PartialEq)]
enum ArgCamera {
    Default,
    Far,
}

impl FromStr for ArgCamera {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(Self::Default),
            "far" => Ok(Self::Far),
            _ => Err("must be 'default' or 'far'".into()),
        }
    }
}

/// `many_morph_targets` stress test
#[derive(FromArgs, Resource)]
struct Args {
    /// number of meshes
    #[argh(option, default = "1024")]
    count: usize,

    /// options: 'animated', 'all', 'none'
    #[argh(option, default = "ArgWeights::Animated")]
    weights: ArgWeights,

    /// options: 'default', 'far'
    #[argh(option, default = "ArgCamera::Default")]
    camera: ArgCamera,
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
                    resolution: WindowResolution::new(1920.0, 1080.0)
                        .with_scale_factor_override(1.0),
                    ..default()
                }),
                ..Default::default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        })
        .insert_resource(AmbientLight {
            brightness: 1000.0,
            ..Default::default()
        })
        .insert_resource(args)
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component, Clone)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
    speed: f32,
}

impl AnimationToPlay {
    fn with_speed(&self, speed: f32) -> Self {
        AnimationToPlay {
            speed,
            ..self.clone()
        }
    }
}

fn setup(
    args: Res<Args>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut commands: Commands,
) {
    const ASSET_PATH: &str = "models/animated/MorphStressTest.gltf";

    let scene = SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(ASSET_PATH)));

    let mut rng = ChaCha8Rng::seed_from_u64(856673);

    let animations = (0..3)
        .map(|gltf_index| {
            let (graph, index) = AnimationGraph::from_clip(
                asset_server.load(GltfAssetLabel::Animation(gltf_index).from_asset(ASSET_PATH)),
            );
            AnimationToPlay {
                graph_handle: graphs.add(graph),
                index,
                speed: 1.0,
            }
        })
        .collect::<Vec<_>>();

    // Arrange the meshes in a grid.

    let count = args.count;
    let x_dim = ((count as f32).sqrt().ceil() as usize).max(1);
    let y_dim = count.div_ceil(x_dim);

    for mesh_index in 0..count {
        let animation = animations[mesh_index.rem_euclid(animations.len())].clone();

        let x = 2.5 + (5.0 * ((mesh_index.rem_euclid(x_dim) as f32) - ((x_dim as f32) * 0.5)));
        let y = -2.2 - (3.0 * ((mesh_index.div_euclid(x_dim) as f32) - ((y_dim as f32) * 0.5)));

        // Randomly vary the animation speed so that the number of morph targets
        // active on each frame is more likely to be stable.

        let animation_speed = rng.r#gen::<f32>() + 0.5;

        commands
            .spawn((
                animation.with_speed(animation_speed),
                scene.clone(),
                Transform::from_xyz(x, y, 0.0),
            ))
            .observe(play_animation)
            .observe(set_weights);
    }

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_rotation_z(PI / 2.0)),
    ));

    let camera_distance = (x_dim as f32)
        * match args.camera {
            ArgCamera::Default => 4.0,
            ArgCamera::Far => 100.0,
        };

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, camera_distance).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn play_animation(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    args: Res<Args>,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if args.weights != ArgWeights::Animated {
        return;
    }

    if let Ok(animation_to_play) = animations_to_play.get(trigger.target()) {
        for child in children.iter_descendants(trigger.target()) {
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
    trigger: Trigger<SceneInstanceReady>,
    args: Res<Args>,
    children: Query<&Children>,
    mut weight_components: Query<&mut MorphWeights>,
) {
    let weight_value = match args.weights {
        ArgWeights::All => Some(1.0),
        ArgWeights::None => Some(0.0),
        _ => None,
    };

    if let Some(weight_value) = weight_value {
        for child in children.iter_descendants(trigger.target()) {
            if let Ok(mut weight_component) = weight_components.get_mut(child) {
                weight_component.weights_mut().fill(weight_value);
            }
        }
    }
}
