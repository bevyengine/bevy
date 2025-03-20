//! TODO

use argh::FromArgs;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    scene::SceneInstanceReady,
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};
use std::f32::consts::PI;

/// TODO
#[derive(FromArgs, Resource)]
struct Args {
    /// TODO
    #[argh(option, default = "1024")]
    count: usize,
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

#[derive(Component)]
struct AnimationToPlay(Handle<AnimationClip>);

fn setup(asset_server: Res<AssetServer>, args: Res<Args>, mut commands: Commands) {
    let scene_handle = asset_server
        .load(GltfAssetLabel::Scene(0).from_asset("models/animated/MorphStressTest.gltf"));

    let animation_handles = (0..3)
        .map(|index| {
            asset_server.load(
                GltfAssetLabel::Animation(index).from_asset("models/animated/MorphStressTest.gltf"),
            )
        })
        .collect::<Vec<_>>();

    let count = args.count.max(1);
    let x_dim = ((count as f32).sqrt().ceil() as usize).max(1);
    let y_dim = count.div_ceil(x_dim);

    for mesh_index in 0..count {
        let animation_index = mesh_index.rem_euclid(animation_handles.len());
        let animation_handle = animation_handles[animation_index].clone();

        let x = 2.5 + (5.0 * ((mesh_index.rem_euclid(x_dim) as f32) - ((x_dim as f32) * 0.5)));
        let y = -2.2 - (3.0 * ((mesh_index.div_euclid(x_dim) as f32) - ((y_dim as f32) * 0.5)));

        commands
            .spawn((
                AnimationToPlay(animation_handle),
                SceneRoot(scene_handle.clone()),
                Transform::from_xyz(x, y, 0.0),
            ))
            .observe(play_animation);
    }

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_rotation_z(PI / 2.0)),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, (x_dim as f32) * 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn play_animation(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    if let Ok(animation_to_play) = animations_to_play.get(trigger.target()) {
        for child in children.iter_descendants(trigger.target()) {
            if let Ok(mut player) = players.get_mut(child) {
                let (graph, animation_index) =
                    AnimationGraph::from_clip(animation_to_play.0.clone());

                commands
                    .entity(child)
                    .insert(AnimationGraphHandle(graphs.add(graph)));

                player.play(animation_index).repeat();
            }
        }
    }
}
