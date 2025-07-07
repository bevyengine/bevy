//! This module contains systems that update the UI when something changes

use crate::{
    experimental::{UiChildren, UiRootNodes},
    ui_transform::UiGlobalTransform,
    CalculatedClip, ComputedNodeTarget, DefaultUiCamera, Display, Node, OverflowAxis, OverrideClip,
    UiScale, UiTargetCamera,
};

use super::ComputedNode;
use bevy_ecs::{
    change_detection::DetectChangesMut,
    entity::Entity,
    hierarchy::ChildOf,
    query::{Changed, Has, With},
    system::{Commands, Query, Res},
};
use bevy_math::{Rect, UVec2};
use bevy_render::camera::Camera;
use bevy_sprite::BorderRect;

/// Updates clipping for all nodes
pub fn update_clipping_system(
    mut commands: Commands,
    root_nodes: UiRootNodes,
    mut node_query: Query<(
        &Node,
        &ComputedNode,
        &UiGlobalTransform,
        Option<&mut CalculatedClip>,
        Has<OverrideClip>,
    )>,
    ui_children: UiChildren,
) {
    for root_node in root_nodes.iter() {
        update_clipping(
            &mut commands,
            &ui_children,
            &mut node_query,
            root_node,
            None,
        );
    }
}

fn update_clipping(
    commands: &mut Commands,
    ui_children: &UiChildren,
    node_query: &mut Query<(
        &Node,
        &ComputedNode,
        &UiGlobalTransform,
        Option<&mut CalculatedClip>,
        Has<OverrideClip>,
    )>,
    entity: Entity,
    mut maybe_inherited_clip: Option<Rect>,
) {
    let Ok((node, computed_node, transform, maybe_calculated_clip, has_override_clip)) =
        node_query.get_mut(entity)
    else {
        return;
    };

    // If the UI node entity has an `OverrideClip` component, discard any inherited clip rect
    if has_override_clip {
        maybe_inherited_clip = None;
    }

    // If `display` is None, clip the entire node and all its descendants by replacing the inherited clip with a default rect (which is empty)
    if node.display == Display::None {
        maybe_inherited_clip = Some(Rect::default());
    }

    // Update this node's CalculatedClip component
    if let Some(mut calculated_clip) = maybe_calculated_clip {
        if let Some(inherited_clip) = maybe_inherited_clip {
            // Replace the previous calculated clip with the inherited clipping rect
            if calculated_clip.clip != inherited_clip {
                *calculated_clip = CalculatedClip {
                    clip: inherited_clip,
                };
            }
        } else {
            // No inherited clipping rect, remove the component
            commands.entity(entity).remove::<CalculatedClip>();
        }
    } else if let Some(inherited_clip) = maybe_inherited_clip {
        // No previous calculated clip, add a new CalculatedClip component with the inherited clipping rect
        commands.entity(entity).try_insert(CalculatedClip {
            clip: inherited_clip,
        });
    }

    // Calculate new clip rectangle for children nodes
    let children_clip = if node.overflow.is_visible() {
        // The current node doesn't clip, propagate the optional inherited clipping rect to any children
        maybe_inherited_clip
    } else {
        // Find the current node's clipping rect and intersect it with the inherited clipping rect, if one exists
        let mut clip_rect = Rect::from_center_size(transform.translation, computed_node.size());

        // Content isn't clipped at the edges of the node but at the edges of the region specified by [`Node::overflow_clip_margin`].
        //
        // `clip_inset` should always fit inside `node_rect`.
        // Even if `clip_inset` were to overflow, we won't return a degenerate result as `Rect::intersect` will clamp the intersection, leaving it empty.
        let clip_inset = match node.overflow_clip_margin.visual_box {
            crate::OverflowClipBox::BorderBox => BorderRect::ZERO,
            crate::OverflowClipBox::ContentBox => computed_node.content_inset(),
            crate::OverflowClipBox::PaddingBox => computed_node.border(),
        };

        clip_rect.min.x += clip_inset.left;
        clip_rect.min.y += clip_inset.top;
        clip_rect.max.x -= clip_inset.right;
        clip_rect.max.y -= clip_inset.bottom;

        clip_rect = clip_rect
            .inflate(node.overflow_clip_margin.margin.max(0.) / computed_node.inverse_scale_factor);

        if node.overflow.x == OverflowAxis::Visible {
            clip_rect.min.x = -f32::INFINITY;
            clip_rect.max.x = f32::INFINITY;
        }
        if node.overflow.y == OverflowAxis::Visible {
            clip_rect.min.y = -f32::INFINITY;
            clip_rect.max.y = f32::INFINITY;
        }
        Some(maybe_inherited_clip.map_or(clip_rect, |c| c.intersect(clip_rect)))
    };

    for child in ui_children.iter_ui_children(entity) {
        update_clipping(commands, ui_children, node_query, child, children_clip);
    }
}

pub fn update_ui_context_system(
    default_ui_camera: DefaultUiCamera,
    ui_scale: Res<UiScale>,
    camera_query: Query<&Camera>,
    target_camera_query: Query<&UiTargetCamera>,
    ui_root_nodes: UiRootNodes,
    mut computed_target_query: Query<&mut ComputedNodeTarget>,
    ui_children: UiChildren,
    reparented_nodes: Query<(Entity, &ChildOf), (Changed<ChildOf>, With<ComputedNodeTarget>)>,
) {
    let default_camera_entity = default_ui_camera.get();

    for root_entity in ui_root_nodes.iter() {
        let camera = target_camera_query
            .get(root_entity)
            .ok()
            .map(UiTargetCamera::entity)
            .or(default_camera_entity)
            .unwrap_or(Entity::PLACEHOLDER);

        let (scale_factor, physical_size) = camera_query
            .get(camera)
            .ok()
            .map(|camera| {
                (
                    camera.target_scaling_factor().unwrap_or(1.) * ui_scale.0,
                    camera.physical_viewport_size().unwrap_or(UVec2::ZERO),
                )
            })
            .unwrap_or((1., UVec2::ZERO));

        update_contexts_recursively(
            root_entity,
            ComputedNodeTarget {
                camera,
                scale_factor,
                physical_size,
            },
            &ui_children,
            &mut computed_target_query,
        );
    }

    for (entity, child_of) in reparented_nodes.iter() {
        let Ok(computed_target) = computed_target_query.get(child_of.parent()) else {
            continue;
        };

        update_contexts_recursively(
            entity,
            *computed_target,
            &ui_children,
            &mut computed_target_query,
        );
    }
}

fn update_contexts_recursively(
    entity: Entity,
    inherited_computed_target: ComputedNodeTarget,
    ui_children: &UiChildren,
    query: &mut Query<&mut ComputedNodeTarget>,
) {
    if query
        .get_mut(entity)
        .map(|mut computed_target| computed_target.set_if_neq(inherited_computed_target))
        .unwrap_or(false)
    {
        for child in ui_children.iter_ui_children(entity) {
            update_contexts_recursively(child, inherited_computed_target, ui_children, query);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_asset::AssetEvent;
    use bevy_asset::Assets;
    use bevy_core_pipeline::core_2d::Camera2d;
    use bevy_ecs::event::Events;
    use bevy_ecs::hierarchy::ChildOf;
    use bevy_ecs::schedule::IntoScheduleConfigs;
    use bevy_ecs::schedule::Schedule;
    use bevy_ecs::world::World;
    use bevy_image::Image;
    use bevy_math::UVec2;
    use bevy_render::camera::Camera;
    use bevy_render::camera::RenderTarget;
    use bevy_render::texture::ManualTextureViews;
    use bevy_utils::default;
    use bevy_window::PrimaryWindow;
    use bevy_window::Window;
    use bevy_window::WindowCreated;
    use bevy_window::WindowRef;
    use bevy_window::WindowResized;
    use bevy_window::WindowResolution;
    use bevy_window::WindowScaleFactorChanged;

    use crate::ComputedNodeTarget;
    use crate::IsDefaultUiCamera;
    use crate::Node;
    use crate::UiScale;
    use crate::UiTargetCamera;

    fn setup_test_world_and_schedule() -> (World, Schedule) {
        let mut world = World::new();

        world.init_resource::<UiScale>();

        // init resources required by `camera_system`
        world.init_resource::<Events<WindowScaleFactorChanged>>();
        world.init_resource::<Events<WindowResized>>();
        world.init_resource::<Events<WindowCreated>>();
        world.init_resource::<Events<AssetEvent<Image>>>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<ManualTextureViews>();

        let mut schedule = Schedule::default();

        schedule.add_systems(
            (
                bevy_render::camera::camera_system,
                super::update_ui_context_system,
            )
                .chain(),
        );

        (world, schedule)
    }

    #[test]
    fn update_context_for_single_ui_root() {
        let (mut world, mut schedule) = setup_test_world_and_schedule();

        let scale_factor = 10.;
        let physical_size = UVec2::new(1000, 500);

        world.spawn((
            Window {
                resolution: WindowResolution::new(physical_size.x as f32, physical_size.y as f32)
                    .with_scale_factor_override(10.),
                ..Default::default()
            },
            PrimaryWindow,
        ));

        let camera = world.spawn(Camera2d).id();

        let uinode = world.spawn(Node::default()).id();

        schedule.run(&mut world);

        assert_eq!(
            *world.get::<ComputedNodeTarget>(uinode).unwrap(),
            ComputedNodeTarget {
                camera,
                physical_size,
                scale_factor,
            }
        );
    }

    #[test]
    fn update_multiple_context_for_multiple_ui_roots() {
        let (mut world, mut schedule) = setup_test_world_and_schedule();

        let scale1 = 1.;
        let size1 = UVec2::new(100, 100);
        let scale2 = 2.;
        let size2 = UVec2::new(200, 200);

        world.spawn((
            Window {
                resolution: WindowResolution::new(size1.x as f32, size1.y as f32)
                    .with_scale_factor_override(scale1),
                ..Default::default()
            },
            PrimaryWindow,
        ));

        let window_2 = world
            .spawn((Window {
                resolution: WindowResolution::new(size2.x as f32, size2.y as f32)
                    .with_scale_factor_override(scale2),
                ..Default::default()
            },))
            .id();

        let camera1 = world.spawn((Camera2d, IsDefaultUiCamera)).id();
        let camera2 = world
            .spawn((
                Camera2d,
                Camera {
                    target: RenderTarget::Window(WindowRef::Entity(window_2)),
                    ..default()
                },
            ))
            .id();

        let uinode1a = world.spawn(Node::default()).id();
        let uinode2a = world.spawn((Node::default(), UiTargetCamera(camera2))).id();
        let uinode2b = world.spawn((Node::default(), UiTargetCamera(camera2))).id();
        let uinode2c = world.spawn((Node::default(), UiTargetCamera(camera2))).id();
        let uinode1b = world.spawn(Node::default()).id();

        schedule.run(&mut world);

        for (uinode, camera, scale_factor, physical_size) in [
            (uinode1a, camera1, scale1, size1),
            (uinode1b, camera1, scale1, size1),
            (uinode2a, camera2, scale2, size2),
            (uinode2b, camera2, scale2, size2),
            (uinode2c, camera2, scale2, size2),
        ] {
            assert_eq!(
                *world.get::<ComputedNodeTarget>(uinode).unwrap(),
                ComputedNodeTarget {
                    camera,
                    scale_factor,
                    physical_size,
                }
            );
        }
    }

    #[test]
    fn update_context_on_changed_camera() {
        let (mut world, mut schedule) = setup_test_world_and_schedule();

        let scale1 = 1.;
        let size1 = UVec2::new(100, 100);
        let scale2 = 2.;
        let size2 = UVec2::new(200, 200);

        world.spawn((
            Window {
                resolution: WindowResolution::new(size1.x as f32, size1.y as f32)
                    .with_scale_factor_override(scale1),
                ..Default::default()
            },
            PrimaryWindow,
        ));

        let window_2 = world
            .spawn((Window {
                resolution: WindowResolution::new(size2.x as f32, size2.y as f32)
                    .with_scale_factor_override(scale2),
                ..Default::default()
            },))
            .id();

        let camera1 = world.spawn((Camera2d, IsDefaultUiCamera)).id();
        let camera2 = world
            .spawn((
                Camera2d,
                Camera {
                    target: RenderTarget::Window(WindowRef::Entity(window_2)),
                    ..default()
                },
            ))
            .id();

        let uinode = world.spawn(Node::default()).id();

        schedule.run(&mut world);

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode)
                .unwrap()
                .scale_factor,
            scale1
        );

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode)
                .unwrap()
                .physical_size,
            size1
        );

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode)
                .unwrap()
                .camera()
                .unwrap(),
            camera1
        );

        world.entity_mut(uinode).insert(UiTargetCamera(camera2));

        schedule.run(&mut world);

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode)
                .unwrap()
                .scale_factor,
            scale2
        );

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode)
                .unwrap()
                .physical_size,
            size2
        );

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode)
                .unwrap()
                .camera()
                .unwrap(),
            camera2
        );
    }

    #[test]
    fn update_context_after_parent_removed() {
        let (mut world, mut schedule) = setup_test_world_and_schedule();

        let scale1 = 1.;
        let size1 = UVec2::new(100, 100);
        let scale2 = 2.;
        let size2 = UVec2::new(200, 200);

        world.spawn((
            Window {
                resolution: WindowResolution::new(size1.x as f32, size1.y as f32)
                    .with_scale_factor_override(scale1),
                ..Default::default()
            },
            PrimaryWindow,
        ));

        let window_2 = world
            .spawn((Window {
                resolution: WindowResolution::new(size2.x as f32, size2.y as f32)
                    .with_scale_factor_override(scale2),
                ..Default::default()
            },))
            .id();

        let camera1 = world.spawn((Camera2d, IsDefaultUiCamera)).id();
        let camera2 = world
            .spawn((
                Camera2d,
                Camera {
                    target: RenderTarget::Window(WindowRef::Entity(window_2)),
                    ..default()
                },
            ))
            .id();

        // `UiTargetCamera` is ignored on non-root UI nodes
        let uinode1 = world.spawn((Node::default(), UiTargetCamera(camera2))).id();
        let uinode2 = world.spawn(Node::default()).add_child(uinode1).id();

        schedule.run(&mut world);

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode1)
                .unwrap()
                .scale_factor(),
            scale1
        );

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode1)
                .unwrap()
                .physical_size(),
            size1
        );

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode1)
                .unwrap()
                .camera()
                .unwrap(),
            camera1
        );

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode2)
                .unwrap()
                .camera()
                .unwrap(),
            camera1
        );

        // Now `uinode1` is a root UI node its `UiTargetCamera` component will be used and its camera target set to `camera2`.
        world.entity_mut(uinode1).remove::<ChildOf>();

        schedule.run(&mut world);

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode1)
                .unwrap()
                .scale_factor(),
            scale2
        );

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode1)
                .unwrap()
                .physical_size(),
            size2
        );

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode1)
                .unwrap()
                .camera()
                .unwrap(),
            camera2
        );

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode2)
                .unwrap()
                .camera()
                .unwrap(),
            camera1
        );
    }

    #[test]
    fn update_great_grandchild() {
        let (mut world, mut schedule) = setup_test_world_and_schedule();

        let scale = 1.;
        let size = UVec2::new(100, 100);

        world.spawn((
            Window {
                resolution: WindowResolution::new(size.x as f32, size.y as f32)
                    .with_scale_factor_override(scale),
                ..Default::default()
            },
            PrimaryWindow,
        ));

        let camera = world.spawn(Camera2d).id();

        let uinode = world.spawn(Node::default()).id();
        world.spawn(Node::default()).with_children(|builder| {
            builder.spawn(Node::default()).with_children(|builder| {
                builder.spawn(Node::default()).add_child(uinode);
            });
        });

        schedule.run(&mut world);

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode)
                .unwrap()
                .scale_factor,
            scale
        );

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode)
                .unwrap()
                .physical_size,
            size
        );

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode)
                .unwrap()
                .camera()
                .unwrap(),
            camera
        );

        world.resource_mut::<UiScale>().0 = 2.;

        schedule.run(&mut world);

        assert_eq!(
            world
                .get::<ComputedNodeTarget>(uinode)
                .unwrap()
                .scale_factor(),
            2.
        );
    }
}
