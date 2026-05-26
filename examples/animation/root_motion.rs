//! Demonstrates the usage of root motion.

use bevy::{
    animation::{
        find_root_bone_recursive, AnimationTargetId, RepeatAnimation, RootMotion, RootMotionConfig,
        RootMotionMode,
    },
    app::AnimationSystems,
    color::palettes::css::SILVER,
    light::CascadeShadowConfigBuilder,
    prelude::*,
    world_serialization::WorldInstanceReady,
};

const MODEL: &str = "models/animated/FoxRootMotion.glb";
const ORIGIN_POSITION: Vec3 = Vec3::new(0., 0., -50.);
const HELP_TEXT: &str = "Press 'Space' to toggle root motion ";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_root_motion)
        // We apply the root motion after the animations are processed, but before propagating the transform
        .add_systems(
            PostUpdate,
            apply_root_motion
                .after(AnimationSystems)
                .before(TransformSystems::Propagate),
        )
        .run();
}

#[derive(Component)]
struct ApplyRootMotionTo(Entity);

#[derive(Component)]
struct HelpTextMarker;

fn apply_root_motion(
    q_root_motion: Query<(&RootMotion, &ApplyRootMotionTo)>,
    mut q_transform: Query<&mut Transform>,
) {
    for (root_motion, apply_to) in q_root_motion {
        let mut transform = q_transform.get_mut(apply_to.0).unwrap();
        // If your model is scaled, you probably want to scale the RootMotion accordingly
        // By default, the RootMotion is not affected by the scale
        let scaled_delta = root_motion.translation_delta * transform.scale;
        transform.translation += scaled_delta;

        // We reset the fox position before it leaves the ground.
        if transform.translation.z > 70. {
            transform.translation = ORIGIN_POSITION;
        }
    }
}

#[derive(Resource)]
struct RootMotionTargetId(AnimationTargetId);

fn toggle_root_motion(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut help_text: Single<&mut Text, With<HelpTextMarker>>,
    mut q_transform: Query<&mut Transform>,
    root_motion_target_id: Option<Res<RootMotionTargetId>>,
    player: Single<(Entity, Option<&RootMotionConfig>, &ApplyRootMotionTo)>,
) {
    if let Some(root_motion_target_id) = root_motion_target_id
        && keys.just_pressed(KeyCode::Space)
    {
        let (player_entity, root_motion_config, apply_to) = player.into_inner();
        match root_motion_config {
            Some(_) => {
                help_text.0 = HELP_TEXT.to_string() + "(current: Off)";
                commands.entity(player_entity).remove::<RootMotionConfig>();
            }
            None => {
                help_text.0 = HELP_TEXT.to_string() + "(current: On)";
                commands.entity(player_entity).insert(RootMotionConfig {
                    root_motion_mode: RootMotionMode::Translation,
                    root_motion_target: root_motion_target_id.0,
                });
            }
        }
        q_transform.get_mut(apply_to.0).unwrap().translation = ORIGIN_POSITION;
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        // Spawn the fox
        .spawn((
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(MODEL))),
            Transform::from_scale(Vec3::splat(0.10)).with_translation(ORIGIN_POSITION),
        ))
        // When the scene is ready, launch the animation and link the scene with the main entity to apply root motion.
        .observe(
            |trigger: On<WorldInstanceReady>,
             q_children: Query<&Children>,
             q_name: Query<&Name>,
             q_animation_target_id: Query<&AnimationTargetId>,
             asset_server: Res<AssetServer>,
             mut q_animation_player: Query<&mut AnimationPlayer>,
             mut animation_graphs: ResMut<Assets<AnimationGraph>>,
             mut commands: Commands| {
                for scene in q_children.get(trigger.event_target()).unwrap().iter() {
                    let Ok(scene_children) = q_children.get(scene) else {
                        continue;
                    };
                    for scene_child in scene_children {
                        if let Ok(mut animation_player) = q_animation_player.get_mut(*scene_child) {
                            let mut animation_graph = AnimationGraph::new();
                            let clip_handle =
                                asset_server.load(GltfAssetLabel::Animation(2).from_asset(MODEL));
                            let animation_node_index =
                                animation_graph.add_clip(clip_handle, 1.0, animation_graph.root);
                            animation_player
                                .play(animation_node_index)
                                .set_repeat(RepeatAnimation::Forever)
                                .set_speed(0.5);
                            commands.entity(*scene_child).insert(AnimationGraphHandle(
                                animation_graphs.add(animation_graph),
                            ));
                            // Here we dig in the rig hierarchy to find the root bone.
                            let root_motion_target_id = find_root_bone_recursive(
                                *scene_child,
                                &q_children,
                                &q_name,
                                &q_animation_target_id,
                                &Name::new("b_Root_00"),
                            )
                            .unwrap();
                            commands.entity(*scene_child).insert((
                                RootMotionConfig {
                                    // You can choose if you want to get Translation + Rotation
                                    // or only Translation with RootMotionMode.
                                    // By default, it's Translation + Rotation.
                                    root_motion_mode: RootMotionMode::Translation,
                                    root_motion_target: root_motion_target_id,
                                },
                                ApplyRootMotionTo(trigger.event_target()),
                            ));
                            commands.insert_resource(RootMotionTargetId(root_motion_target_id));
                            return;
                        }
                    }
                }
                error!("Animation Player wasn't found");
            },
        );

    // Some light to see something
    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.,
            shadow_maps_enabled: true,
            ..Default::default()
        },
        CascadeShadowConfigBuilder {
            maximum_distance: 500.,
            ..Default::default()
        }
        .build(),
        Transform::from_xyz(8., 16., 8.).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50., 150.))),
        MeshMaterial3d(materials.add(Color::from(SILVER))),
    ));

    // The camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-60., 60., 100.).looking_at(Vec3::new(0., 0., 30.), Vec3::Y),
    ));
    // Help Text
    commands.spawn((
        HelpTextMarker,
        Text::new(HELP_TEXT.to_string() + "(current: On)"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}
