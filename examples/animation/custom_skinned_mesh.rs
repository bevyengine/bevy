//! Skinned mesh example with mesh and joints data defined in code.
//! Example taken from <https://github.com/KhronosGroup/glTF-Tutorials/blob/master/gltfTutorial/gltfTutorial_019_SimpleSkin.md>

use std::f32::consts::*;

use bevy::{
    prelude::*,
    render::{
        mesh::{
            skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
            Indices, PrimitiveTopology, VertexAttributeValues,
        },
        render_asset::RenderAssetUsages,
    },
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            brightness: 3000.0,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, joint_animation)
        .run();
}

/// Used to mark a joint to be animated in the [`joint_animation`] system.
#[derive(Component)]
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
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Create inverse bindpose matrices for a skeleton consists of 2 joints
    let inverse_bindposes = skinned_mesh_inverse_bindposes_assets.add(vec![
        Mat4::from_translation(Vec3::new(-0.5, -1.0, 0.0)),
        Mat4::from_translation(Vec3::new(-0.5, -1.0, 0.0)),
    ]);

    // Create a mesh
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    // Set mesh vertex positions
    .with_inserted_attribute(
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
    )
    // Set mesh vertex normals
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 10])
    // Set mesh vertex joint indices for mesh skinning.
    // Each vertex gets 4 indices used to address the `JointTransforms` array in the vertex shader
    //  as well as `SkinnedMeshJoint` array in the `SkinnedMesh` component.
    // This means that a maximum of 4 joints can affect a single vertex.
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_JOINT_INDEX,
        // Need to be explicit here as [u16; 4] could be either Uint16x4 or Unorm16x4.
        VertexAttributeValues::Uint16x4(vec![
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
        ]),
    )
    // Set mesh vertex joint weights for mesh skinning.
    // Each vertex gets 4 joint weights corresponding to the 4 joint indices assigned to it.
    // The sum of these weights should equal to 1.
    .with_inserted_attribute(
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
    )
    // Tell bevy to construct triangles from a list of vertex indices,
    //  where each 3 vertex indices form an triangle.
    .with_inserted_indices(Indices::U16(vec![
        0, 1, 3, 0, 3, 2, 2, 3, 5, 2, 5, 4, 4, 5, 7, 4, 7, 6, 6, 7, 9, 6, 9, 8,
    ]));

    let mesh = meshes.add(mesh);

    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let mut rng = ChaCha8Rng::seed_from_u64(42);

    for i in -5..5 {
        // Create joint entities
        let joint_0 = commands
            .spawn(TransformBundle::from(Transform::from_xyz(
                i as f32 * 1.5,
                0.0,
                i as f32 * 0.1,
            )))
            .id();
        let joint_1 = commands
            .spawn((AnimatedJoint, TransformBundle::IDENTITY))
            .id();

        // Set joint_1 as a child of joint_0.
        commands.entity(joint_0).push_children(&[joint_1]);

        // Each joint in this vector corresponds to each inverse bindpose matrix in `SkinnedMeshInverseBindposes`.
        let joint_entities = vec![joint_0, joint_1];

        // Create skinned mesh renderer. Note that its transform doesn't affect the position of the mesh.
        commands.spawn((
            PbrBundle {
                mesh: mesh.clone(),
                material: materials.add(Color::srgb(
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.0..1.0),
                )),
                ..default()
            },
            SkinnedMesh {
                inverse_bindposes: inverse_bindposes.clone(),
                joints: joint_entities,
            },
        ));
    }
}

/// Animate the joint marked with [`AnimatedJoint`] component.
fn joint_animation(time: Res<Time>, mut query: Query<&mut Transform, With<AnimatedJoint>>) {
    for mut transform in &mut query {
        transform.rotation = Quat::from_rotation_z(FRAC_PI_2 * time.elapsed_seconds().sin());
    }
}
