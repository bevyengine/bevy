//! Test `SkinnedMeshBounds` by showing the bounds of various animated meshes.

use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::DynamicSkinnedMeshBounds,
    mesh::{
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        PrimitiveTopology, VertexAttributeValues,
    },
    prelude::*,
    scene::SceneInstanceReady,
};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Test Skinned Mesh Bounds".into(),
                ..default()
            }),
            ..default()
        }))
        .insert_gizmo_config(
            SkinnedMeshBoundsGizmoConfigGroup {
                draw_all: true,
                ..Default::default()
            },
            GizmoConfig::default(),
        )
        .insert_gizmo_config(
            AabbGizmoConfigGroup {
                draw_all: true,
                ..Default::default()
            },
            GizmoConfig::default(),
        )
        .insert_resource(GlobalAmbientLight {
            brightness: 2000.0,
            ..Default::default()
        })
        .add_systems(Startup, setup)
        .add_systems(Startup, load_scene)
        .add_systems(Update, spawn_scene)
        .add_systems(Startup, spawn_custom_meshes)
        .add_systems(Update, update_custom_mesh_animation)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 7.5, 18.0).looking_at(Vec3::new(0.0, 5.5, 0.0), Vec3::Y),
    ));
}

#[derive(Component, Debug, Default)]
struct PendingScene(Handle<Gltf>);

#[derive(Component, Debug, Default)]
struct PendingAnimation((Handle<AnimationGraph>, AnimationNodeIndex));

fn load_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        PendingScene(asset_server.load("models/animated/Fox.glb")),
        Transform::from_xyz(1.3, 4.3, 0.0)
            .with_scale(Vec3::splat(0.08))
            .looking_to(-Vec3::X, Vec3::Y),
    ));
}

fn spawn_scene(
    mut commands: Commands,
    query: Query<(Entity, &PendingScene)>,
    assets: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (entity, PendingScene(asset)) in query.iter() {
        if let Some(gltf) = assets.get(asset)
            && let Some(scene_handle) = gltf.scenes.first()
            && let Some(animation_handle) = gltf.named_animations.get("Run")
        {
            let (graph, graph_node_index) = AnimationGraph::from_clip(animation_handle.clone());

            commands
                .entity(entity)
                .remove::<PendingScene>()
                .insert((
                    SceneRoot(scene_handle.clone()),
                    PendingAnimation((graphs.add(graph), graph_node_index)),
                ))
                .observe(play_animation);
        }
    }
}

fn play_animation(
    trigger: On<SceneInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    animations: Query<&PendingAnimation>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if let Ok(PendingAnimation((graph_handle, graph_node_index))) = animations.get(trigger.entity) {
        for child in children.iter_descendants(trigger.entity) {
            if let Ok(mut player) = players.get_mut(child) {
                player.play(*graph_node_index).set_speed(0.6).repeat();

                commands
                    .entity(child)
                    .insert(AnimationGraphHandle(graph_handle.clone()));
            }
        }
    }

    commands.entity(trigger.entity).remove::<PendingAnimation>();
}

type CustomAnimationId = i8;

#[derive(Component)]
struct CustomAnimation(CustomAnimationId);

fn spawn_custom_meshes(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let mesh_handle = mesh_assets.add(
        Mesh::new(
            PrimitiveTopology::TriangleStrip,
            // Test that skinned mesh bounds work even if the mesh is render
            // world only.
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [-0.5, 0.0, 0.0],
                [0.5, 0.0, 0.0],
                [-0.5, 0.5, 0.0],
                [0.5, 0.5, 0.0],
                [-0.5, 1.0, 0.0],
                [0.5, 1.0, 0.0],
                [-0.5, 1.5, 0.0],
                [0.5, 1.5, 0.0],
                [-0.5, 2.0, 0.0],
                [0.5, 2.0, 0.0],
            ],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 10])
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![
                [1, 0, 0, 0],
                [1, 0, 0, 0],
                [1, 2, 0, 0],
                [1, 2, 0, 0],
                [1, 2, 0, 0],
                [1, 2, 0, 0],
                [2, 1, 0, 0],
                [2, 1, 0, 0],
                [2, 0, 0, 0],
                [2, 0, 0, 0],
            ]),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            vec![
                [1.00, 0.00, 0.0, 0.0],
                [1.00, 0.00, 0.0, 0.0],
                [0.75, 0.25, 0.0, 0.0],
                [0.75, 0.25, 0.0, 0.0],
                [0.50, 0.50, 0.0, 0.0],
                [0.50, 0.50, 0.0, 0.0],
                [0.75, 0.25, 0.0, 0.0],
                [0.75, 0.25, 0.0, 0.0],
                [1.00, 0.00, 0.0, 0.0],
                [1.00, 0.00, 0.0, 0.0],
            ],
        )
        .with_generated_skinned_mesh_bounds()
        .unwrap(),
    );

    let inverse_bindposes_handle = inverse_bindposes_assets.add(vec![
        Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
    ]);

    struct MeshInstance {
        animations: [CustomAnimationId; 2],
    }

    let mesh_instances = [
        // Simple cases. First joint is still, second joint is all rotation/translation/scale variations.
        MeshInstance { animations: [0, 1] },
        MeshInstance { animations: [0, 2] },
        MeshInstance { animations: [0, 3] },
        MeshInstance { animations: [0, 4] },
        MeshInstance { animations: [0, 5] },
        MeshInstance { animations: [0, 6] },
        MeshInstance { animations: [0, 7] },
        MeshInstance { animations: [0, 8] },
        // Skewed cases. First joint is non-uniform scaling, second joint is rotation/translation variations.
        MeshInstance { animations: [9, 1] },
        MeshInstance { animations: [9, 2] },
        MeshInstance { animations: [9, 3] },
        MeshInstance { animations: [9, 4] },
        MeshInstance { animations: [9, 5] },
    ];

    for (i, mesh_instance) in mesh_instances.iter().enumerate() {
        let x = ((i as f32) * 2.0) - ((mesh_instances.len() - 1) as f32);

        let base_entity = commands
            .spawn((Transform::from_xyz(x, 0.0, 0.0), Visibility::default()))
            .id();

        let joints = vec![
            commands.spawn((Transform::IDENTITY,)).id(),
            commands
                .spawn((
                    CustomAnimation(mesh_instance.animations[0]),
                    Transform::IDENTITY,
                ))
                .id(),
            commands
                .spawn((
                    CustomAnimation(mesh_instance.animations[1]),
                    Transform::IDENTITY,
                ))
                .id(),
        ];

        commands.entity(joints[0]).insert(ChildOf(base_entity));

        commands.entity(joints[1]).insert(ChildOf(joints[0]));
        commands.entity(joints[2]).insert(ChildOf(joints[1]));

        let mesh_entity = commands
            .spawn((
                Transform::IDENTITY,
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(material_assets.add(StandardMaterial {
                    base_color: Color::WHITE,
                    cull_mode: None,
                    ..default()
                })),
                SkinnedMesh {
                    inverse_bindposes: inverse_bindposes_handle.clone(),
                    joints: joints.clone(),
                },
                DynamicSkinnedMeshBounds,
            ))
            .id();

        commands.entity(mesh_entity).insert(ChildOf(base_entity));
    }
}

fn update_custom_mesh_animation(
    time: Res<Time<Virtual>>,
    mut query: Query<(&mut Transform, &CustomAnimation)>,
) {
    let t = time.elapsed_secs();
    let ts = ops::sin(t);
    let tc = ops::cos(t);
    let ots = ops::sin(t + FRAC_PI_4);
    let otc = ops::cos(t + FRAC_PI_4);

    for (mut transform, animation) in &mut query {
        match animation.0 {
            1 => transform.translation = Vec3::new(0.5 * ts, 0.3 + tc, 0.0),
            2 => transform.translation = Vec3::new(0.0, 0.5 + ts, tc),
            3 => transform.rotation = Quat::from_rotation_x(FRAC_PI_2 * ts),
            4 => transform.rotation = Quat::from_rotation_y(FRAC_PI_2 * ts),
            5 => transform.rotation = Quat::from_rotation_z(FRAC_PI_2 * ts),
            6 => transform.scale.x = ts * 1.5,
            7 => transform.scale.y = ts * 1.5,
            8 => transform.scale = Vec3::new(ts * 1.5, otc * 1.5, 1.0),
            9 => transform.scale = Vec3::new(ots, 1.0 + (tc * 0.3), 1.0 - (tc * 0.5)),
            _ => (),
        }
    }
}
