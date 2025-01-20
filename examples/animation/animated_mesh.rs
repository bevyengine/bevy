//! Plays an animation on a skinned glTF model of a fox.

use std::f32::consts::PI;

use bevy::{pbr::CascadeShadowConfigBuilder, prelude::*};

// An example asset that contains a mesh and animation.
const GLTF_PATH: &str = "models/animated/Fox.glb";

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_mesh_and_animation)
        .add_systems(Startup, setup_camera_and_environment)
        .add_systems(Update, play_animation_once_loaded)
        .run();
}

#[derive(Resource)]
struct Animations {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

// Create an animation graph and start loading the mesh and animation.
fn setup_mesh_and_animation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Build an animation graph containing a single animation.
    let (graph, index) = AnimationGraph::from_clip(
        // We want the "run" animation from our example asset, which has an
        // index of two.
        asset_server.load(GltfAssetLabel::Animation(2).from_asset(GLTF_PATH)),
    );

    // Keep our animation graph in a Resource so that it can be added to the
    // correct entity once the scene loads.
    let graph_handle = graphs.add(graph);
    commands.insert_resource(Animations {
        graph_handle,
        index,
    });

    // Tell the engine to start loading our mesh and animation, and then spawn
    // them as a scene when ready.
    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset(GLTF_PATH)),
    ));
}

// Detect that the scene is loaded and spawned, then play the animation.
fn play_animation_once_loaded(
    mut commands: Commands,
    animations: Res<Animations>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
) {
    for (entity, mut player) in &mut players {
        // Start the animation player and tell it to repeat forever.
        //
        // If you want to try stopping and switching animations, see the
        // `animated_mesh_control.rs` example.
        player.play(animations.index).repeat();

        // Insert the animation graph with our selected animation. This
        // connects the animation player to the mesh.
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animations.graph_handle.clone()));
    }
}

// Spawn a camera and a simple environment with a ground plane and light.
fn setup_camera_and_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(100.0, 100.0, 150.0).looking_at(Vec3::new(0.0, 20.0, 0.0), Vec3::Y),
    ));

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(500000.0, 500000.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // Light
    commands.spawn((
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 200.0,
            maximum_distance: 400.0,
            ..default()
        }
        .build(),
    ));
}
