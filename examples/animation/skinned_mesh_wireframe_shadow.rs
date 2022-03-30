use std::f32::consts::PI;

use bevy::{
    pbr::{
        wireframe::{WireframeConfig, WireframePlugin},
        AmbientLight,
    },
    prelude::*,
    render::{mesh::skinning::SkinnedMesh, render_resource::WgpuFeatures, settings::WgpuSettings},
};

/// Skinned mesh example with mesh and joints data loaded from a glTF file.
/// Example taken from <https://github.com/KhronosGroup/glTF-Tutorials/blob/master/gltfTutorial/gltfTutorial_019_SimpleSkin.md>
fn main() {
    App::new()
        .insert_resource(WgpuSettings {
            features: WgpuFeatures::POLYGON_MODE_LINE,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(WireframeConfig { global: true })
        .add_plugin(WireframePlugin)
        .insert_resource(AmbientLight {
            brightness: 1.0,
            ..Default::default()
        })
        .add_startup_system(setup)
        .add_system(joint_animation)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create a camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 50.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    // Spawn the first scene in `models/SimpleSkin/SimpleSkin.gltf`
    commands.spawn_scene(asset_server.load::<Scene, _>("models/SimpleSkin/SimpleSkin.gltf#Scene0"));

    commands.spawn_bundle(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            1.0,
            -std::f32::consts::FRAC_PI_4,
        )),
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: -10.0,
                right: 10.0,
                bottom: -10.0,
                top: 10.0,
                near: -10.0,
                far: 10.0,
                ..default()
            },
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });
}

/// The scene hierachy currently looks somewhat like this:
///
/// ```ignore
/// <Parent entity>
///   + Mesh node (without `PbrBundle` or `SkinnedMesh` component)
///     + Skinned mesh entity (with `PbrBundle` and `SkinnedMesh` component, created by glTF loader)
///     + First joint
///       + Second joint
/// ```
///
/// In this example, we want to get and animate the second joint.
/// It is similar to the animation defined in `models/SimpleSkin/SimpleSkin.gltf`.
fn joint_animation(
    time: Res<Time>,
    parent_query: Query<&Parent, With<SkinnedMesh>>,
    children_query: Query<&Children>,
    mut transform_query: Query<&mut Transform>,
) {
    // Iter skinned mesh entity
    for skinned_mesh_parent in parent_query.iter() {
        // Mesh node is the parent of the skinned mesh entity.
        let mesh_node_entity = skinned_mesh_parent.0;
        // Get `Children` in the mesh node.
        let mesh_node_children = children_query.get(mesh_node_entity).unwrap();

        // First joint is the second child of the mesh node.
        let first_joint_entity = mesh_node_children[1];
        // Get `Children` in the first joint.
        let first_joint_children = children_query.get(first_joint_entity).unwrap();

        // Second joint is the first child of the first joint.
        let second_joint_entity = first_joint_children[0];
        // Get `Transform` in the second joint.
        let mut second_joint_transform = transform_query.get_mut(second_joint_entity).unwrap();

        second_joint_transform.rotation = Quat::from_axis_angle(
            Vec3::Z,
            0.5 * PI * time.time_since_startup().as_secs_f32().sin(),
        );
    }
}
