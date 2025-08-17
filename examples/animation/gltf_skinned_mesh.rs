//! Skinned mesh example with mesh and joints data loaded from a glTF file.
//! Example taken from <https://github.com/KhronosGroup/glTF-Tutorials/blob/master/gltfTutorial/gltfTutorial_019_SimpleSkin.md>

use std::f32::consts::*;

use bevy::{math::ops, mesh::skinning::SkinnedMesh, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            brightness: 750.0,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, joint_animation)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Create a camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // Spawn the first scene in `models/SimpleSkin/SimpleSkin.gltf`
    commands.spawn(SceneRoot(asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("models/SimpleSkin/SimpleSkin.gltf"),
    )));
}

/// The scene hierarchy currently looks somewhat like this:
///
/// ```text
/// <Parent entity>
///   + Mesh node (without `Mesh3d` or `SkinnedMesh` component)
///     + Skinned mesh entity (with `Mesh3d` and `SkinnedMesh` component, created by glTF loader)
///     + First joint
///       + Second joint
/// ```
///
/// In this example, we want to get and animate the second joint.
/// It is similar to the animation defined in `models/SimpleSkin/SimpleSkin.gltf`.
fn joint_animation(
    time: Res<Time>,
    children: Query<&ChildOf, With<SkinnedMesh>>,
    parents: Query<&Children>,
    mut transform_query: Query<&mut Transform>,
) {
    // Iter skinned mesh entity
    for child_of in &children {
        // Mesh node is the parent of the skinned mesh entity.
        let mesh_node_entity = child_of.parent();
        // Get `Children` in the mesh node.
        let mesh_node_parent = parents.get(mesh_node_entity).unwrap();

        // First joint is the second child of the mesh node.
        let first_joint_entity = mesh_node_parent[1];
        // Get `Children` in the first joint.
        let first_joint_children = parents.get(first_joint_entity).unwrap();

        // Second joint is the first child of the first joint.
        let second_joint_entity = first_joint_children[0];
        // Get `Transform` in the second joint.
        let mut second_joint_transform = transform_query.get_mut(second_joint_entity).unwrap();

        second_joint_transform.rotation =
            Quat::from_rotation_z(FRAC_PI_2 * ops::sin(time.elapsed_secs()));
    }
}
