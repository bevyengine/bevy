use std::f32::consts::PI;

use bevy::{
    pbr::AmbientLight,
    prelude::*,
    render::{
        mesh::Indices,
        pipeline::{PrimitiveTopology, RenderPipeline},
    },
};

/// Skinned mesh example with mesh and joints data defined in code.
/// Example taken from https://github.com/KhronosGroup/glTF-Tutorials/blob/master/gltfTutorial/gltfTutorial_019_SimpleSkin.md
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            brightness: 1.0,
            ..Default::default()
        })
        .add_startup_system(setup.system())
        .add_system(joint_animation.system())
        .run();
}

/// Used to mark a joint to be animated in the [`joint_animation`] system.
struct AnimatedJoint;

/// Construct a mesh and a skeleton with 2 joints for that mesh,
///   and mark the second joint to be animated.
/// It is similar to the scene defined in `models/SimpleSkin/SimpleSkin.gltf`
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut skinned_mesh_inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    // Create a camera
    let mut camera = OrthographicCameraBundle::new_2d();
    camera.orthographic_projection.near = -1.0;
    camera.orthographic_projection.far = 1.0;
    camera.orthographic_projection.scale = 0.005;
    camera.transform = Transform::from_xyz(0.0, 1.0, 0.0);
    commands.spawn_bundle(camera);

    // Create inverse bindpose matrices for a skeleton consists of 2 joints
    let inverse_bindposes =
        skinned_mesh_inverse_bindposes_assets.add(SkinnedMeshInverseBindposes(vec![
            Mat4::from_translation(Vec3::new(-0.5, -1.0, 0.0)),
            Mat4::from_translation(Vec3::new(-0.5, -1.0, 0.0)),
        ]));

    // Create a mesh
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    // Set mesh vertex positions
    mesh.set_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.5, 0.0],
            [1.0, 0.5, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.5, 0.0],
            [1.0, 1.5, 0.0],
            [0.0, 2.0, 0.0],
            [1.0, 2.0, 0.0],
        ],
    );
    // Set mesh vertex normals
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 10]);
    // Set mesh vertex UVs. Although the mesh doesn't have any texture applied,
    //  UVs are still required by the render pipeline. So these UVs are zeroed out.
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; 10]);
    // Set mesh vertex joint indices for mesh skinning.
    // Each vertex gets 4 indices used to address the `JointTransforms` array in the vertex shader
    //  as well as `SkinnedMeshJoint` array in the `SkinnedMesh` component.
    // This means that a maximum of 4 joints can affect a single vertex.
    mesh.set_attribute(
        Mesh::ATTRIBUTE_JOINT_INDEX,
        vec![
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 1, 0, 0],
            [0, 1, 0, 0],
            [0, 1, 0, 0],
            [0, 1, 0, 0],
            [0, 1, 0, 0],
            [0, 1, 0, 0],
            [0, 1, 0, 0],
            [0, 1, 0, 0],
        ],
    );
    // Set mesh vertex joint weights for mesh skinning.
    // Each vertex gets 4 joint weights corresponding to the 4 joint indices assigned to it.
    // The sum of these weights should equal to 1.
    mesh.set_attribute(
        Mesh::ATTRIBUTE_JOINT_WEIGHT,
        vec![
            [1.00, 0.00, 0.0, 0.0],
            [1.00, 0.00, 0.0, 0.0],
            [0.75, 0.25, 0.0, 0.0],
            [0.75, 0.25, 0.0, 0.0],
            [0.50, 0.50, 0.0, 0.0],
            [0.50, 0.50, 0.0, 0.0],
            [0.25, 0.75, 0.0, 0.0],
            [0.25, 0.75, 0.0, 0.0],
            [0.00, 1.00, 0.0, 0.0],
            [0.00, 1.00, 0.0, 0.0],
        ],
    );
    // Tell bevy to construct triangles from a list of vertex indices,
    //  where each 3 vertex indices form an triangle.
    mesh.set_indices(Some(Indices::U16(vec![
        0, 1, 3, 0, 3, 2, 2, 3, 5, 2, 5, 4, 4, 5, 7, 4, 7, 6, 6, 7, 9, 6, 9, 8,
    ])));

    // Create joint entities
    let joint_0 = commands
        .spawn_bundle((
            Transform::from_xyz(0.0, 1.0, 0.0),
            GlobalTransform::identity(),
        ))
        .id();
    let joint_1 = commands
        .spawn_bundle((
            AnimatedJoint,
            Transform::identity(),
            GlobalTransform::identity(),
        ))
        .id();

    // Set joint_1 as a child of joint_0.
    commands.entity(joint_0).push_children(&[joint_1]);

    // Each joint in this vector corresponds to each inverse bindpose matrix in `SkinnedMeshInverseBindposes`.
    let joint_entities = vec![joint_0, joint_1];

    // Create skinned mesh renderer. Note that its transform doesn't affect the position of the mesh.
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(mesh),
            material: materials.add(Color::WHITE.into()),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                SKINNED_MESH_PIPELINE_HANDLE.typed(),
            )]),
            ..Default::default()
        })
        .insert(SkinnedMesh::new(inverse_bindposes, joint_entities));
}

/// Animate the joint marked with [`AnimatedJoint`] component.
fn joint_animation(time: Res<Time>, mut query: Query<&mut Transform, With<AnimatedJoint>>) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_axis_angle(
            Vec3::Z,
            0.5 * PI * time.time_since_startup().as_secs_f32().sin(),
        );
    }
}
