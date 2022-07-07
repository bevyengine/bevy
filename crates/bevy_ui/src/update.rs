//! This module contains systems that update the UI when something changes

use crate::{
    prelude::{CameraUiConfig, UiCameraRenderInfo},
    CalculatedClip, Overflow, Style, UI_CAMERA_FAR,
};

use super::Node;
use bevy_ecs::{
    entity::Entity,
    prelude::{ChangeTrackers, Changed, Component, Or},
    query::{With, Without},
    system::{Commands, Query},
};
use bevy_hierarchy::{Children, Parent};
use bevy_math::Vec2;
use bevy_render::camera::{
    Camera, CameraProjection, DepthCalculation, OrthographicProjection, WindowOrigin,
};
use bevy_sprite::Rect;
use bevy_transform::components::{GlobalTransform, Transform};

/// The resolution of Z values for UI
pub const UI_Z_STEP: f32 = 0.001;

/// Updates transforms of nodes to fit with the z system
pub fn ui_z_system(
    root_node_query: Query<Entity, (With<Node>, Without<Parent>)>,
    mut node_query: Query<&mut Transform, With<Node>>,
    children_query: Query<&Children>,
) {
    let mut current_global_z = 0.0;
    for entity in root_node_query.iter() {
        current_global_z = update_hierarchy(
            &children_query,
            &mut node_query,
            entity,
            current_global_z,
            current_global_z,
        );
    }
}

fn update_hierarchy(
    children_query: &Query<&Children>,
    node_query: &mut Query<&mut Transform, With<Node>>,
    entity: Entity,
    parent_global_z: f32,
    mut current_global_z: f32,
) -> f32 {
    current_global_z += UI_Z_STEP;
    if let Ok(mut transform) = node_query.get_mut(entity) {
        let new_z = current_global_z - parent_global_z;
        // only trigger change detection when the new value is different
        if transform.translation.z != new_z {
            transform.translation.z = new_z;
        }
    }
    if let Ok(children) = children_query.get(entity) {
        let current_parent_global_z = current_global_z;
        for child in children.iter().cloned() {
            current_global_z = update_hierarchy(
                children_query,
                node_query,
                child,
                current_parent_global_z,
                current_global_z,
            );
        }
    }
    current_global_z
}

/// Updates clipping for all nodes
pub fn update_clipping_system(
    mut commands: Commands,
    root_node_query: Query<Entity, (With<Node>, Without<Parent>)>,
    mut node_query: Query<(&Node, &GlobalTransform, &Style, Option<&mut CalculatedClip>)>,
    children_query: Query<&Children>,
) {
    for root_node in root_node_query.iter() {
        update_clipping(
            &mut commands,
            &children_query,
            &mut node_query,
            root_node,
            None,
        );
    }
}

fn update_clipping(
    commands: &mut Commands,
    children_query: &Query<&Children>,
    node_query: &mut Query<(&Node, &GlobalTransform, &Style, Option<&mut CalculatedClip>)>,
    entity: Entity,
    clip: Option<Rect>,
) {
    let (node, global_transform, style, calculated_clip) = node_query.get_mut(entity).unwrap();
    // Update this node's CalculatedClip component
    match (clip, calculated_clip) {
        (None, None) => {}
        (None, Some(_)) => {
            commands.entity(entity).remove::<CalculatedClip>();
        }
        (Some(clip), None) => {
            commands.entity(entity).insert(CalculatedClip { clip });
        }
        (Some(clip), Some(mut old_clip)) => {
            *old_clip = CalculatedClip { clip };
        }
    }

    // Calculate new clip for its children
    let children_clip = match style.overflow {
        Overflow::Visible => clip,
        Overflow::Hidden => {
            let node_center = global_transform.translation.truncate();
            let node_rect = Rect {
                min: node_center - node.size / 2.,
                max: node_center + node.size / 2.,
            };
            if let Some(clip) = clip {
                Some(Rect {
                    min: Vec2::max(clip.min, node_rect.min),
                    max: Vec2::min(clip.max, node_rect.max),
                })
            } else {
                Some(node_rect)
            }
        }
    };

    if let Ok(children) = children_query.get(entity) {
        for child in children.iter().cloned() {
            update_clipping(commands, children_query, node_query, child, children_clip);
        }
    }
}

pub fn update_ui_camera_data<T: Component>(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &Camera,
            Option<&CameraUiConfig>,
            Option<&mut UiCameraRenderInfo>,
            Option<ChangeTrackers<CameraUiConfig>>,
        ),
        (With<T>, Or<(Changed<Camera>, Changed<CameraUiConfig>)>),
    >,
) {
    for (entity, camera, config, render_info, config_changed) in query.iter_mut() {
        if matches!(config, Some(&CameraUiConfig { show_ui: false, .. })) {
            commands.entity(entity).remove::<UiCameraRenderInfo>();
            continue;
        }
        let logical_size = if let Some(logical_size) = camera.logical_viewport_size() {
            logical_size
        } else {
            commands.entity(entity).remove::<UiCameraRenderInfo>();
            continue;
        };
        // skip work if there is no changes.
        if let (Some(projection), Some(config_changed)) = (&render_info, config_changed) {
            if projection.old_logical_size == logical_size && !config_changed.is_changed() {
                continue;
            }
        }

        let (view_pos, scale) = if let Some(config) = config {
            (config.position, config.scale)
        } else {
            (Vec2::new(0.0, 0.0), 1.0)
        };
        let mut new_projection = OrthographicProjection {
            far: UI_CAMERA_FAR,
            scale,
            window_origin: WindowOrigin::BottomLeft,
            depth_calculation: DepthCalculation::ZDifference,
            ..Default::default()
        };
        new_projection.update(logical_size.x, logical_size.y);
        if let Some(mut info) = render_info {
            info.projection = new_projection;
            info.position = view_pos;
            info.old_logical_size = logical_size;
        } else {
            commands.entity(entity).insert(UiCameraRenderInfo {
                projection: new_projection,
                position: view_pos,
                old_logical_size: logical_size,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        component::Component,
        schedule::{Schedule, Stage, SystemStage},
        system::{CommandQueue, Commands},
        world::World,
    };
    use bevy_hierarchy::BuildChildren;
    use bevy_transform::components::Transform;

    use crate::Node;

    use super::{ui_z_system, UI_Z_STEP};

    #[derive(Component, PartialEq, Debug, Clone)]
    struct Label(&'static str);

    fn node_with_transform(name: &'static str) -> (Label, Node, Transform) {
        (Label(name), Node::default(), Transform::identity())
    }

    fn node_without_transform(name: &'static str) -> (Label, Node) {
        (Label(name), Node::default())
    }

    fn get_steps(transform: &Transform) -> u32 {
        (transform.translation.z / UI_Z_STEP).round() as u32
    }

    #[test]
    fn test_ui_z_system() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        commands.spawn_bundle(node_with_transform("0"));

        commands
            .spawn_bundle(node_with_transform("1"))
            .with_children(|parent| {
                parent
                    .spawn_bundle(node_with_transform("1-0"))
                    .with_children(|parent| {
                        parent.spawn_bundle(node_with_transform("1-0-0"));
                        parent.spawn_bundle(node_without_transform("1-0-1"));
                        parent.spawn_bundle(node_with_transform("1-0-2"));
                    });
                parent.spawn_bundle(node_with_transform("1-1"));
                parent
                    .spawn_bundle(node_without_transform("1-2"))
                    .with_children(|parent| {
                        parent.spawn_bundle(node_with_transform("1-2-0"));
                        parent.spawn_bundle(node_with_transform("1-2-1"));
                        parent
                            .spawn_bundle(node_with_transform("1-2-2"))
                            .with_children(|_| ());
                        parent.spawn_bundle(node_with_transform("1-2-3"));
                    });
                parent.spawn_bundle(node_with_transform("1-3"));
            });

        commands
            .spawn_bundle(node_without_transform("2"))
            .with_children(|parent| {
                parent
                    .spawn_bundle(node_with_transform("2-0"))
                    .with_children(|_parent| ());
                parent
                    .spawn_bundle(node_with_transform("2-1"))
                    .with_children(|parent| {
                        parent.spawn_bundle(node_with_transform("2-1-0"));
                    });
            });
        queue.apply(&mut world);

        let mut schedule = Schedule::default();
        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(ui_z_system);
        schedule.add_stage("update", update_stage);
        schedule.run(&mut world);

        let mut actual_result = world
            .query::<(&Label, &Transform)>()
            .iter(&world)
            .map(|(name, transform)| (name.clone(), get_steps(transform)))
            .collect::<Vec<(Label, u32)>>();
        actual_result.sort_unstable_by_key(|(name, _)| name.0);
        let expected_result = vec![
            (Label("0"), 1),
            (Label("1"), 1),
            (Label("1-0"), 1),
            (Label("1-0-0"), 1),
            // 1-0-1 has no transform
            (Label("1-0-2"), 3),
            (Label("1-1"), 5),
            // 1-2 has no transform
            (Label("1-2-0"), 1),
            (Label("1-2-1"), 2),
            (Label("1-2-2"), 3),
            (Label("1-2-3"), 4),
            (Label("1-3"), 11),
            // 2 has no transform
            (Label("2-0"), 1),
            (Label("2-1"), 2),
            (Label("2-1-0"), 1),
        ];
        assert_eq!(actual_result, expected_result);
    }
}
