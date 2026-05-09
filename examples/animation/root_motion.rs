//! Demonstrates the usage of root motion.

use bevy::{
    animation::AnimationTargetId, color::palettes::css::SILVER, light::CascadeShadowConfigBuilder,
    prelude::*, world_serialization::WorldInstanceReady,
};
use bevy_animation::{RepeatAnimation, RootMotion, RootMotionMode};

const MODEL: &'static str = "models/animated/FoxRootMotion.glb";
const ORIGIN_POSITION: Vec3 = Vec3::new(0., 0., -50.);
const HELP_TEXT: &'static str = "Press 'Space' to toggle root motion ";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (apply_root_motion, toggle_root_motion))
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
        let scaled_delta = Vec3 {
            x: root_motion.translation_delta.x * transform.scale.x,
            y: root_motion.translation_delta.y * transform.scale.y,
            z: root_motion.translation_delta.z * transform.scale.z,
        };
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
    keys: Res<ButtonInput<KeyCode>>,
    mut help_text: Single<&mut Text, With<HelpTextMarker>>,
    mut q_transform: Query<&mut Transform>,
    root_motion_target_id: Option<Res<RootMotionTargetId>>,
    player: Single<(&mut AnimationPlayer, &ApplyRootMotionTo)>,
) {
    if let Some(root_motion_target_id) = root_motion_target_id {
        if keys.just_pressed(KeyCode::Space) {
            let (mut animation_player, apply_to) = player.into_inner();
            if animation_player.root_motion_target().is_none() {
                animation_player.set_root_motion_target(Some(root_motion_target_id.0));
                help_text.0 = HELP_TEXT.to_string() + "(current: On)";
            } else {
                animation_player.set_root_motion_target(None);
                help_text.0 = HELP_TEXT.to_string() + "(current: Off)";
            }
            q_transform.get_mut(apply_to.0).unwrap().translation = ORIGIN_POSITION;
        }
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
                            // You can choose if you want to get Translation + Rotation
                            // or only Translation with RootMotionMode.
                            // By default, it's Translation + Rotation.
                            animation_player.set_root_motion_mode(RootMotionMode::Translation);
                            commands
                                .entity(*scene_child)
                                .insert(ApplyRootMotionTo(trigger.event_target()));
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
        Text::new(HELP_TEXT.to_string() + "(current: Off)"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

/// Finds the root node so we can configure the [`AnimationPlayer`] to extract the motion from it.
fn find_root_bone_recursive(
    entity: Entity,
    q_children: &Query<&Children>,
    q_name: &Query<&Name>,
    q_animation_target_id: &Query<&AnimationTargetId>,
    name: &Name,
) -> Option<AnimationTargetId> {
    if let Ok(entity_name) = q_name.get(entity)
        && name == entity_name
    {
        return q_animation_target_id.get(entity).ok().copied();
    }
    if let Ok(children) = q_children.get(entity) {
        for child in children {
            let found =
                find_root_bone_recursive(*child, q_children, q_name, q_animation_target_id, name);
            if found.is_some() {
                return found;
            }
        }
    }
    None
}
