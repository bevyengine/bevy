//! Play an animation with morph targets.
//!
//! Also illustrates how to read morph target names in `name_morphs`.

use argh::FromArgs;
use bevy::{mesh::morph::MeshMorphWeights, pbr::CacheSkin, prelude::*, scene::SceneInstanceReady};
use std::f32::consts::PI;

const GLTF_PATH: &str = "models/animated/MorphStressTest.gltf";

/// plays an animation with morphs
#[derive(FromArgs, Resource)]
struct Args {
    /// enable skin caching
    #[argh(switch)]
    cache_skins: bool,
}

fn main() {
    // `from_env` panics on the web
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GlobalAmbientLight {
            brightness: 150.0,
            ..default()
        })
        .insert_resource(args)
        .add_systems(Startup, setup)
        .add_systems(Update, name_morphs)
        .add_systems(
            Update,
            mark_skins_as_cached.run_if(|args: Res<Args>| args.cache_skins),
        )
        .run();
}

#[derive(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let (graph, index) = AnimationGraph::from_clip(
        asset_server.load(GltfAssetLabel::Animation(2).from_asset(GLTF_PATH)),
    );

    commands
        .spawn((
            AnimationToPlay {
                graph_handle: graphs.add(graph),
                index,
            },
            SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(GLTF_PATH))),
        ))
        .observe(play_animation_when_ready);

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_rotation_z(PI / 2.0)),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 2.1, 10.2).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn play_animation_when_ready(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations_to_play: Query<&AnimationToPlay>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if let Ok(animation_to_play) = animations_to_play.get(scene_ready.entity) {
        for child in children.iter_descendants(scene_ready.entity) {
            if let Ok(mut player) = players.get_mut(child) {
                player.play(animation_to_play.index).repeat();

                commands
                    .entity(child)
                    .insert(AnimationGraphHandle(animation_to_play.graph_handle.clone()));
            }
        }
    }
}

/// Adds `CacheSkin` components to morphed meshes if skin caching was requested
/// on the command line.
fn mark_skins_as_cached(
    mut commands: Commands,
    query: Query<Entity, (With<MeshMorphWeights>, Without<CacheSkin>)>,
) {
    for entity in &query {
        commands.entity(entity).insert(CacheSkin);
    }
}

/// Whenever a mesh asset is loaded, print the name of the asset and the names
/// of its morph targets.
fn name_morphs(
    asset_server: Res<AssetServer>,
    mut events: MessageReader<AssetEvent<Mesh>>,
    meshes: Res<Assets<Mesh>>,
) {
    for event in events.read() {
        if let AssetEvent::<Mesh>::Added { id } = event
            && let Some(path) = asset_server.get_path(*id)
            && let Some(mesh) = meshes.get(*id)
            && let Some(names) = mesh.morph_target_names()
        {
            info!("Morph target names for {path:?}:");

            for name in names {
                info!("  {name}");
            }
        }
    }
}
