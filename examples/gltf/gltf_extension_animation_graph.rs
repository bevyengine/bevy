//! Uses glTF extension processing to play an animation on a skinned glTF model of a fox.

use std::f32::consts::PI;

use bevy::{
    asset::LoadContext,
    ecs::entity::EntityHashSet,
    gltf::extensions::{GltfExtensionHandler, GltfExtensionHandlers},
    light::CascadeShadowConfigBuilder,
    platform::collections::{HashMap, HashSet},
    prelude::*,
    scene::SceneInstanceReady,
};

/// An example asset that contains a mesh and animation.
const GLTF_PATH: &str = "models/animated/Fox.glb";

fn main() {
    App::new()
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
            ..default()
        })
        .add_plugins((DefaultPlugins, GltfExtensionHandlerAnimationPlugin))
        .add_systems(
            Startup,
            (setup_mesh_and_animation, setup_camera_and_environment),
        )
        .run();
}

/// A component that stores a reference to an animation we want to play. This is
/// created when we start loading the mesh (see `setup_mesh_and_animation`) and
/// read when the mesh has spawned (see `play_animation_once_loaded`).
#[derive(Component, Reflect)]
#[reflect(Component)]
struct AnimationToPlay {
    graph_handle: Handle<AnimationGraph>,
    index: AnimationNodeIndex,
}

fn setup_mesh_and_animation(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn an entity with our components, and connect it to an observer that
    // will trigger when the scene is loaded and spawned.
    commands
        .spawn(SceneRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset(GLTF_PATH)),
        ))
        .observe(play_animation_when_ready);
}

fn play_animation_when_ready(
    scene_ready: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    mut players: Query<(&mut AnimationPlayer, &AnimationToPlay)>,
) {
    for child in children.iter_descendants(scene_ready.entity) {
        let Ok((mut player, animation_to_play)) = players.get_mut(child) else {
            continue;
        };

        // Tell the animation player to start the animation and keep
        // repeating it.
        //
        // If you want to try stopping and switching animations, see the
        // `animated_mesh_control.rs` example.
        player.play(animation_to_play.index).repeat();

        // Add the animation graph. This only needs to be done once to
        // connect the animation player to the mesh.
        commands
            .entity(child)
            .insert(AnimationGraphHandle(animation_to_play.graph_handle.clone()));
    }
}

/// Spawn a camera and a simple environment with a ground plane and light.
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

struct GltfExtensionHandlerAnimationPlugin;

impl Plugin for GltfExtensionHandlerAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut()
            .resource_mut::<GltfExtensionHandlers>()
            .0
            .write_blocking()
            .push(Box::new(GltfExtensionHandlerAnimation::default()));
    }
}

#[derive(Default, Clone)]
struct GltfExtensionHandlerAnimation {
    animation_root_indices: HashSet<usize>,
    animation_root_entities: EntityHashSet,
    clip: Option<Handle<AnimationClip>>,
}

impl GltfExtensionHandler for GltfExtensionHandlerAnimation {
    fn dyn_clone(&self) -> Box<dyn GltfExtensionHandler> {
        Box::new((*self).clone())
    }

    #[cfg(feature = "bevy_animation")]
    fn on_animation(&mut self, gltf_animation: &gltf::Animation, handle: Handle<AnimationClip>) {
        if gltf_animation.name().is_some_and(|v| v == "Walk") {
            self.clip = Some(handle.clone());
        }
    }
    #[cfg(feature = "bevy_animation")]
    fn on_animations_collected(
        &mut self,
        _load_context: &mut LoadContext<'_>,
        _animations: &[Handle<AnimationClip>],
        _named_animations: &HashMap<Box<str>, Handle<AnimationClip>>,
        animation_roots: &HashSet<usize>,
    ) {
        self.animation_root_indices = animation_roots.clone();
    }

    fn on_gltf_node(
        &mut self,
        _load_context: &mut LoadContext<'_>,
        gltf_node: &gltf::Node,
        entity: &mut EntityWorldMut,
    ) {
        if self.animation_root_indices.contains(&gltf_node.index()) {
            self.animation_root_entities.insert(entity.id());
        }
    }

    /// Called when an individual Scene is done processing
    fn on_scene_completed(
        &mut self,
        load_context: &mut LoadContext<'_>,
        _scene: &gltf::Scene,
        _world_root_id: Entity,
        world: &mut World,
    ) {
        // Create an AnimationGraph from the desired clip
        let (graph, index) = AnimationGraph::from_clip(self.clip.clone().unwrap());
        // Store the animation graph as an asset with an arbitrary label
        // We only have one graph, so this label will be unique
        let graph_handle =
            load_context.add_labeled_asset("MyAnimationGraphLabel".to_string(), graph);

        // Create a component that stores a reference to our animation
        let animation_to_play = AnimationToPlay {
            graph_handle,
            index,
        };

        // Insert the `AnimationToPlay` component on the first animation root
        let mut entity = world.entity_mut(*self.animation_root_entities.iter().next().unwrap());
        entity.insert(animation_to_play);
    }
}
