use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_math::{Quat, Vec3};
use bevy_utils::HashMap;

mod loader;
pub use loader::*;

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, Handle};
use bevy_pbr::StandardMaterial;
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::mesh::Mesh;
use bevy_scene::Scene;

/// Adds support for glTF file loading to the app.
#[derive(Default)]
pub struct GltfPlugin;

impl Plugin for GltfPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<GltfLoader>()
            .add_asset::<Gltf>()
            .add_asset::<GltfNode>()
            .add_asset::<GltfPrimitive>()
            .add_asset::<GltfMesh>()
            .add_asset::<GltfAnimation>()
            .register_type::<GltfAnimatedNode>();
    }
}

/// Representation of a loaded glTF file.
#[derive(Debug, TypeUuid)]
#[uuid = "5c7d5f8a-f7b0-4e45-a09e-406c0372fea2"]
pub struct Gltf {
    pub scenes: Vec<Handle<Scene>>,
    pub named_scenes: HashMap<String, Handle<Scene>>,
    pub meshes: Vec<Handle<GltfMesh>>,
    pub named_meshes: HashMap<String, Handle<GltfMesh>>,
    pub materials: Vec<Handle<StandardMaterial>>,
    pub named_materials: HashMap<String, Handle<StandardMaterial>>,
    pub nodes: Vec<Handle<GltfNode>>,
    pub named_nodes: HashMap<String, Handle<GltfNode>>,
    pub default_scene: Option<Handle<Scene>>,
    pub animations: Vec<Handle<GltfAnimation>>,
    pub named_animations: HashMap<String, Handle<GltfAnimation>>,
}

/// A glTF node with all of its child nodes, its [`GltfMesh`] and
/// [`Transform`](bevy_transform::prelude::Transform).
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "dad74750-1fd6-460f-ac51-0a7937563865"]
pub struct GltfNode {
    pub children: Vec<GltfNode>,
    pub mesh: Option<Handle<GltfMesh>>,
    pub transform: bevy_transform::prelude::Transform,
}

/// A glTF mesh, which may consists of multiple [`GtlfPrimitives`](GltfPrimitive).
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "8ceaec9a-926a-4f29-8ee3-578a69f42315"]
pub struct GltfMesh {
    pub primitives: Vec<GltfPrimitive>,
}

/// Part of a [`GltfMesh`] that consists of a [`Mesh`] and an optional [`StandardMaterial`].
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "cbfca302-82fd-41cb-af77-cab6b3d50af1"]
pub struct GltfPrimitive {
    pub mesh: Handle<Mesh>,
    pub material: Option<Handle<StandardMaterial>>,
}

/// Interpolation method for an animation. Part of a [`GltfNodeAnimation`].
#[derive(Clone, Debug)]
pub enum GltfAnimationInterpolation {
    Linear,
    Step,
    CubicSpline,
}

/// How a property of a glTF node should be animated. The property and its value can be found
/// through the [`GltfNodeAnimationKeyframes`] attribute.
#[derive(Clone, Debug)]
pub struct GltfNodeAnimation {
    pub keyframe_timestamps: Vec<f32>,
    pub keyframes: GltfNodeAnimationKeyframes,
    pub interpolation: GltfAnimationInterpolation,
}

/// A glTF animation, listing how each node (by its index) that is part of it should be animated.
#[derive(Default, Clone, TypeUuid, Debug)]
#[uuid = "d81b7179-0448-4eb0-89fe-c067222725bf"]
pub struct GltfAnimation {
    pub node_animations: HashMap<usize, Vec<GltfNodeAnimation>>,
}

/// Key frames of an animation.
#[derive(Clone, Debug)]
pub enum GltfNodeAnimationKeyframes {
    Rotation(Vec<Quat>),
    Translation(Vec<Vec3>),
    Scale(Vec<Vec3>),
}

impl Default for GltfNodeAnimation {
    fn default() -> Self {
        Self {
            keyframe_timestamps: Default::default(),
            keyframes: GltfNodeAnimationKeyframes::Translation(Default::default()),
            interpolation: GltfAnimationInterpolation::Linear,
        }
    }
}

/// A glTF node that is part of an animation, with its index.
#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component)]
pub struct GltfAnimatedNode {
    pub index: usize,
}
