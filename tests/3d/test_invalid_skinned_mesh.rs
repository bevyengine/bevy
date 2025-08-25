//! Test that the renderer can handle various invalid skinned meshes

use bevy::{
    asset::RenderAssetUsages,
    camera::ScalingMode,
    core_pipeline::motion_blur::MotionBlur,
    math::ops,
    mesh::{
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        Indices, PrimitiveTopology, VertexAttributeValues,
    },
    prelude::*,
};
use core::f32::consts::TAU;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            brightness: 20_000.0,
            ..default()
        })
        .add_systems(Startup, (setup_environment, setup_meshes))
        .add_systems(Update, update_animated_joints)
        .run();
}

fn setup_environment(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    let description = "(left to right)\n\
        0: Normal skinned mesh.\n\
        1: Mesh asset is missing skinning attributes.\n\
        2: One joint entity is missing.\n\
        3: Mesh entity is missing SkinnedMesh component.";

    commands.spawn((
        Text::new(description),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 19.0,
                min_height: 6.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        // Add motion blur so we can check if it's working for skinned meshes.
        // This also exercises the renderer's prepass path.
        MotionBlur {
            // Use an unrealistically large shutter angle so that motion blur is clearly visible.
            shutter_angle: 3.0,
            samples: 2,
        },
        // MSAA and MotionBlur together are not compatible on WebGL.
        #[cfg(all(feature = "webgl2", target_arch = "wasm32", not(feature = "webgpu")))]
        Msaa::Off,
    ));

    // Add a directional light to make sure we exercise the renderer's shadow path.
    commands.spawn((
        Transform::from_xyz(1.0, 1.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
    ));

    // Add a plane behind the meshes so we can see the shadows.
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, -1.0),
        Mesh3d(mesh_assets.add(Plane3d::default().mesh().size(100.0, 100.0).normal(Dir3::Z))),
        MeshMaterial3d(material_assets.add(StandardMaterial {
            base_color: Color::srgb(0.05, 0.05, 0.15),
            reflectance: 0.2,
            ..default()
        })),
    ));
}

fn setup_meshes(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    // Create a mesh with two rectangles.
    let unskinned_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.3, -0.3, 0.0],
            [0.3, -0.3, 0.0],
            [-0.3, 0.3, 0.0],
            [0.3, 0.3, 0.0],
            [-0.4, 0.8, 0.0],
            [0.4, 0.8, 0.0],
            [-0.4, 1.8, 0.0],
            [0.4, 1.8, 0.0],
        ],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 8])
    .with_inserted_indices(Indices::U16(vec![0, 1, 3, 0, 3, 2, 4, 5, 7, 4, 7, 6]));

    // Copy the mesh and add skinning attributes that bind each rectangle to a joint.
    let skinned_mesh = unskinned_mesh
        .clone()
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [1, 0, 0, 0],
                [1, 0, 0, 0],
                [1, 0, 0, 0],
                [1, 0, 0, 0],
            ]),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            vec![[1.00, 0.00, 0.0, 0.0]; 8],
        );

    let unskinned_mesh_handle = mesh_assets.add(unskinned_mesh);
    let skinned_mesh_handle = mesh_assets.add(skinned_mesh);

    let inverse_bindposes_handle = inverse_bindposes_assets.add(vec![
        Mat4::IDENTITY,
        Mat4::from_translation(Vec3::new(0.0, -1.3, 0.0)),
    ]);

    let mesh_material_handle = material_assets.add(StandardMaterial::default());

    let background_material_handle = material_assets.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.15, 0.05),
        reflectance: 0.2,
        ..default()
    });

    #[derive(PartialEq)]
    enum Variation {
        Normal,
        MissingMeshAttributes,
        MissingJointEntity,
        MissingSkinnedMeshComponent,
    }

    for (index, variation) in [
        Variation::Normal,
        Variation::MissingMeshAttributes,
        Variation::MissingJointEntity,
        Variation::MissingSkinnedMeshComponent,
    ]
    .into_iter()
    .enumerate()
    {
        // Skip variations that are currently broken. See https://github.com/bevyengine/bevy/issues/16929,
        // https://github.com/bevyengine/bevy/pull/18074.
        if (variation == Variation::MissingSkinnedMeshComponent)
            || (variation == Variation::MissingMeshAttributes)
        {
            continue;
        }

        let transform = Transform::from_xyz(((index as f32) - 1.5) * 4.5, 0.0, 0.0);

        let joint_0 = commands.spawn(transform).id();

        let joint_1 = commands
            .spawn((ChildOf(joint_0), AnimatedJoint, Transform::IDENTITY))
            .id();

        if variation == Variation::MissingJointEntity {
            commands.entity(joint_1).despawn();
        }

        let mesh_handle = match variation {
            Variation::MissingMeshAttributes => &unskinned_mesh_handle,
            _ => &skinned_mesh_handle,
        };

        let mut entity_commands = commands.spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(mesh_material_handle.clone()),
            transform,
        ));

        if variation != Variation::MissingSkinnedMeshComponent {
            entity_commands.insert(SkinnedMesh {
                inverse_bindposes: inverse_bindposes_handle.clone(),
                joints: vec![joint_0, joint_1],
            });
        }

        // Add a square behind the mesh to distinguish it from the other meshes.
        commands.spawn((
            Transform::from_xyz(transform.translation.x, transform.translation.y, -0.8),
            Mesh3d(mesh_assets.add(Plane3d::default().mesh().size(4.3, 4.3).normal(Dir3::Z))),
            MeshMaterial3d(background_material_handle.clone()),
        ));
    }
}

#[derive(Component)]
struct AnimatedJoint;

fn update_animated_joints(time: Res<Time>, query: Query<&mut Transform, With<AnimatedJoint>>) {
    for mut transform in query {
        let angle = TAU * 4.0 * ops::cos((time.elapsed_secs() / 8.0) * TAU);
        let rotation = Quat::from_rotation_z(angle);

        transform.rotation = rotation;
        transform.translation = rotation.mul_vec3(Vec3::new(0.0, 1.3, 0.0));
    }
}
