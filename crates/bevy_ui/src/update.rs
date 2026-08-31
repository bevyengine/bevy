//! This module contains systems that update the UI when something changes

use crate::{
    experimental::{UiChildren, UiRootNodes},
    layout_tree::ComputedLayout,
    ui_transform::UiGlobalTransform,
    CalculatedClip, ComputedUiRenderTargetInfo, ComputedUiTargetCamera, DefaultUiCamera, Display,
    FixedNode, Node, OverrideClip, UiScale, UiTargetCamera, UiTreeChanged,
};

use super::ComputedNode;
use bevy_app::Propagate;
use bevy_camera::Camera;
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    hierarchy::ChildOf,
    query::{Has, Or, With},
    system::{Commands, Query, Res},
    world::Ref,
};
use bevy_math::UVec2;

/// Updates clipping for all nodes
pub fn update_clipping_system(
    mut commands: Commands,
    root_nodes: UiRootNodes,
    fixed_nodes_query: Query<Entity, (With<FixedNode>, With<ChildOf>)>,
    mut node_query: Query<(
        &Node,
        &ComputedNode,
        &ComputedLayout,
        &UiGlobalTransform,
        Option<&mut CalculatedClip>,
        Has<OverrideClip>,
        Has<FixedNode>,
        Ref<UiTreeChanged>,
    )>,
    ui_children: UiChildren,
) {
    for root_node in root_nodes.iter().chain(fixed_nodes_query.iter()) {
        update_clipping(
            &mut commands,
            &ui_children,
            &mut node_query,
            root_node,
            None,
            false,
            true,
        );
    }
}

fn update_clipping(
    commands: &mut Commands,
    ui_children: &UiChildren,
    node_query: &mut Query<(
        &Node,
        &ComputedNode,
        &ComputedLayout,
        &UiGlobalTransform,
        Option<&mut CalculatedClip>,
        Has<OverrideClip>,
        Has<FixedNode>,
        Ref<UiTreeChanged>,
    )>,
    entity: Entity,
    mut maybe_inherited_clip: Option<CalculatedClip>,
    force_update: bool,
    is_root: bool,
) {
    let Ok((
        node,
        computed_node,
        computed_layout,
        transform,
        maybe_calculated_clip,
        has_override_clip,
        has_fixed_node,
        tree_changed,
    )) = node_query.get_mut(entity)
    else {
        return;
    };

    if has_fixed_node && !is_root {
        return;
    }

    if !force_update && !tree_changed.is_changed() {
        return;
    }

    if !force_update && !computed_layout.layout_changed() && !computed_layout.subtree_dirty() {
        return;
    }

    // If the UI node entity has an `OverrideClip`, discard any inherited clip rect
    if has_override_clip {
        maybe_inherited_clip = None;
    }

    // If `display` is None, clip the entire node and all its descendants.
    if node.display == Display::None {
        maybe_inherited_clip = Some(CalculatedClip::FullyClipped);
    }

    // Update this node's CalculatedClip component
    if let Some(mut calculated_clip) = maybe_calculated_clip {
        if let Some(inherited_clip) = maybe_inherited_clip.as_ref() {
            // Replace the previous calculated clip with the inherited clipping rect
            if *calculated_clip != *inherited_clip {
                *calculated_clip = inherited_clip.clone();
            }
        } else {
            // No inherited clipping rect, remove the component
            commands.entity(entity).remove::<CalculatedClip>();
        }
    } else if let Some(inherited_clip) = maybe_inherited_clip.as_ref() {
        // No previous calculated clip, add a new CalculatedClip component with the inherited clipping rect
        commands.entity(entity).try_insert(inherited_clip.clone());
    }

    // Calculate new clip rectangle for children nodes
    let children_clip = if maybe_inherited_clip
        .as_ref()
        .is_some_and(CalculatedClip::is_fully_clipped)
        || node.overflow.is_visible()
    {
        // The current node doesn't clip, propagate the optional inherited clipping rect to any children
        maybe_inherited_clip
    } else if let Some(clip_from_world) = transform.try_inverse() {
        let mut clip = maybe_inherited_clip.unwrap_or_default();
        clip.push_rect(
            computed_node.resolve_clip_rect(node.overflow, node.overflow_clip_margin),
            clip_from_world,
        );
        Some(clip)
    } else {
        Some(CalculatedClip::FullyClipped)
    };

    let propagated_force_update =
        force_update || computed_layout.layout_changed() || computed_layout.self_dirty();
    for child in ui_children.iter_ui_children(entity) {
        update_clipping(
            commands,
            ui_children,
            node_query,
            child,
            children_clip.clone(),
            propagated_force_update,
            false,
        );
    }
}

pub fn propagate_ui_target_cameras(
    mut commands: Commands,
    default_ui_camera: DefaultUiCamera,
    ui_scale: Res<UiScale>,
    camera_query: Query<&Camera>,
    target_camera_query: Query<&UiTargetCamera>,
    ui_root_nodes: UiRootNodes,
    ui_children: UiChildren,
    propagate_query: Query<
        Entity,
        Or<(
            With<Propagate<ComputedUiTargetCamera>>,
            With<Propagate<ComputedUiRenderTargetInfo>>,
        )>,
    >,
) {
    let default_camera_entity = default_ui_camera.get();

    for entity in propagate_query.iter() {
        if ui_children.get_parent(entity).is_some() {
            commands.entity(entity).remove::<(
                Propagate<ComputedUiTargetCamera>,
                Propagate<ComputedUiRenderTargetInfo>,
            )>();
        }
    }

    for root_entity in ui_root_nodes.iter() {
        let camera = target_camera_query
            .get(root_entity)
            .ok()
            .map(UiTargetCamera::entity)
            .or(default_camera_entity);

        commands
            .entity(root_entity)
            .try_insert(Propagate(ComputedUiTargetCamera { camera }));

        let (scale_factor, physical_size) = camera
            .and_then(|camera| camera_query.get(camera).ok())
            .map(|camera| {
                (
                    camera.target_scaling_factor().unwrap_or(1.) * ui_scale.0,
                    camera.physical_viewport_size().unwrap_or(UVec2::ZERO),
                )
            })
            .unwrap_or((1., UVec2::ZERO));

        commands
            .entity(root_entity)
            .try_insert(Propagate(ComputedUiRenderTargetInfo {
                scale_factor,
                physical_size,
            }));
    }
}

#[cfg(test)]
mod tests {
    use crate::update::{propagate_ui_target_cameras, update_clipping_system};
    use crate::CalculatedClip;
    use crate::ComputedUiRenderTargetInfo;
    use crate::ComputedUiTargetCamera;
    use crate::FixedNode;
    use crate::IsDefaultUiCamera;
    use crate::Node;
    use crate::Overflow;
    use crate::OverrideClip;
    use crate::UiScale;
    use crate::UiTargetCamera;
    use bevy_app::App;
    use bevy_app::HierarchyPropagatePlugin;
    use bevy_app::PostUpdate;
    use bevy_app::PropagateSet;
    use bevy_camera::Camera;
    use bevy_camera::Camera2d;
    use bevy_camera::ComputedCameraValues;
    use bevy_camera::RenderTargetInfo;
    use bevy_ecs::hierarchy::ChildOf;
    use bevy_math::UVec2;
    use bevy_utils::default;

    fn setup_test_app() -> App {
        let mut app = App::new();

        app.init_resource::<UiScale>();

        app.add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
            PostUpdate,
        ));
        app.configure_sets(
            PostUpdate,
            PropagateSet::<ComputedUiTargetCamera>::default(),
        );

        app.add_plugins(HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(
            PostUpdate,
        ));
        app.configure_sets(
            PostUpdate,
            PropagateSet::<ComputedUiRenderTargetInfo>::default(),
        );

        app.add_systems(bevy_app::Update, propagate_ui_target_cameras);

        app
    }

    #[test]
    fn update_context_for_single_ui_root() {
        let mut app = setup_test_app();
        let world = app.world_mut();

        let scale_factor = 10.;
        let physical_size = UVec2::new(1000, 500);

        let camera = world
            .spawn((
                Camera2d,
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size,
                            scale_factor,
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .id();

        let uinode = world.spawn(Node::default()).id();

        app.update();
        let world = app.world_mut();

        assert_eq!(
            *world.get::<ComputedUiTargetCamera>(uinode).unwrap(),
            ComputedUiTargetCamera {
                camera: Some(camera)
            }
        );

        assert_eq!(
            *world.get::<ComputedUiRenderTargetInfo>(uinode).unwrap(),
            ComputedUiRenderTargetInfo {
                physical_size,
                scale_factor,
            }
        );
    }

    #[test]
    fn update_multiple_context_for_multiple_ui_roots() {
        let mut app = setup_test_app();
        let world = app.world_mut();

        let scale1 = 1.;
        let size1 = UVec2::new(100, 100);
        let scale2 = 2.;
        let size2 = UVec2::new(200, 200);

        let camera1 = world
            .spawn((
                Camera2d,
                IsDefaultUiCamera,
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size: size1,
                            scale_factor: scale1,
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .id();
        let camera2 = world
            .spawn((
                Camera2d,
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size: size2,
                            scale_factor: scale2,
                        }),
                        ..Default::default()
                    },
                    ..default()
                },
            ))
            .id();

        let uinode1a = world.spawn(Node::default()).id();
        let uinode2a = world.spawn((Node::default(), UiTargetCamera(camera2))).id();
        let uinode2b = world.spawn((Node::default(), UiTargetCamera(camera2))).id();
        let uinode2c = world.spawn((Node::default(), UiTargetCamera(camera2))).id();
        let uinode1b = world.spawn(Node::default()).id();

        app.update();
        let world = app.world_mut();

        for (uinode, camera, scale_factor, physical_size) in [
            (uinode1a, camera1, scale1, size1),
            (uinode1b, camera1, scale1, size1),
            (uinode2a, camera2, scale2, size2),
            (uinode2b, camera2, scale2, size2),
            (uinode2c, camera2, scale2, size2),
        ] {
            assert_eq!(
                *world.get::<ComputedUiTargetCamera>(uinode).unwrap(),
                ComputedUiTargetCamera {
                    camera: Some(camera)
                }
            );

            assert_eq!(
                *world.get::<ComputedUiRenderTargetInfo>(uinode).unwrap(),
                ComputedUiRenderTargetInfo {
                    physical_size,
                    scale_factor,
                }
            );
        }
    }

    #[test]
    fn update_context_on_changed_camera() {
        let mut app = setup_test_app();
        let world = app.world_mut();

        let scale1 = 1.;
        let size1 = UVec2::new(100, 100);
        let scale2 = 2.;
        let size2 = UVec2::new(200, 200);

        let camera1 = world
            .spawn((
                Camera2d,
                IsDefaultUiCamera,
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size: size1,
                            scale_factor: scale1,
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .id();
        let camera2 = world
            .spawn((
                Camera2d,
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size: size2,
                            scale_factor: scale2,
                        }),
                        ..Default::default()
                    },
                    ..default()
                },
            ))
            .id();

        let uinode = world.spawn(Node::default()).id();

        app.update();
        let world = app.world_mut();

        assert_eq!(
            world
                .get::<ComputedUiRenderTargetInfo>(uinode)
                .unwrap()
                .scale_factor,
            scale1
        );

        assert_eq!(
            world
                .get::<ComputedUiRenderTargetInfo>(uinode)
                .unwrap()
                .physical_size,
            size1
        );

        assert_eq!(
            world
                .get::<ComputedUiTargetCamera>(uinode)
                .unwrap()
                .get()
                .unwrap(),
            camera1
        );

        world.entity_mut(uinode).insert(UiTargetCamera(camera2));

        app.update();
        let world = app.world_mut();

        assert_eq!(
            world
                .get::<ComputedUiRenderTargetInfo>(uinode)
                .unwrap()
                .scale_factor,
            scale2
        );

        assert_eq!(
            world
                .get::<ComputedUiRenderTargetInfo>(uinode)
                .unwrap()
                .physical_size,
            size2
        );

        assert_eq!(
            world
                .get::<ComputedUiTargetCamera>(uinode)
                .unwrap()
                .get()
                .unwrap(),
            camera2
        );
    }

    #[test]
    fn update_context_after_parented() {
        let mut app = setup_test_app();
        let world = app.world_mut();

        let camera1 = world.spawn((Camera2d, IsDefaultUiCamera)).id();
        let camera2 = world.spawn(Camera2d).id();
        let parent = world.spawn((Node::default(), UiTargetCamera(camera2))).id();
        let child = world.spawn(Node::default()).id();

        app.update();

        assert_eq!(
            app.world()
                .get::<ComputedUiTargetCamera>(child)
                .unwrap()
                .get(),
            Some(camera1)
        );

        app.world_mut().entity_mut(parent).add_child(child);
        app.update();

        assert_eq!(
            app.world()
                .get::<ComputedUiTargetCamera>(child)
                .unwrap()
                .get(),
            Some(camera2)
        );
    }

    #[test]
    fn update_context_after_parent_removed() {
        let mut app = setup_test_app();
        let world = app.world_mut();

        let scale1 = 1.;
        let size1 = UVec2::new(100, 100);
        let scale2 = 2.;
        let size2 = UVec2::new(200, 200);

        let camera1 = world
            .spawn((
                Camera2d,
                IsDefaultUiCamera,
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size: size1,
                            scale_factor: scale1,
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .id();
        let camera2 = world
            .spawn((
                Camera2d,
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size: size2,
                            scale_factor: scale2,
                        }),
                        ..Default::default()
                    },
                    ..default()
                },
            ))
            .id();

        // `UiTargetCamera` is ignored on non-root UI nodes
        let uinode1 = world.spawn((Node::default(), UiTargetCamera(camera2))).id();
        let uinode2 = world.spawn(Node::default()).add_child(uinode1).id();

        app.update();
        let world = app.world_mut();

        assert_eq!(
            world
                .get::<ComputedUiRenderTargetInfo>(uinode1)
                .unwrap()
                .scale_factor(),
            scale1
        );

        assert_eq!(
            world
                .get::<ComputedUiRenderTargetInfo>(uinode1)
                .unwrap()
                .physical_size(),
            size1
        );

        assert_eq!(
            world
                .get::<ComputedUiTargetCamera>(uinode1)
                .unwrap()
                .get()
                .unwrap(),
            camera1
        );

        assert_eq!(
            world
                .get::<ComputedUiTargetCamera>(uinode2)
                .unwrap()
                .get()
                .unwrap(),
            camera1
        );

        // Now `uinode1` is a root UI node its `UiTargetCamera` component will be used and its camera target set to `camera2`.
        world.entity_mut(uinode1).remove::<ChildOf>();

        app.update();
        let world = app.world_mut();

        assert_eq!(
            world
                .get::<ComputedUiRenderTargetInfo>(uinode1)
                .unwrap()
                .scale_factor(),
            scale2
        );

        assert_eq!(
            world
                .get::<ComputedUiRenderTargetInfo>(uinode1)
                .unwrap()
                .physical_size(),
            size2
        );

        assert_eq!(
            world
                .get::<ComputedUiTargetCamera>(uinode1)
                .unwrap()
                .get()
                .unwrap(),
            camera2
        );

        assert_eq!(
            world
                .get::<ComputedUiTargetCamera>(uinode2)
                .unwrap()
                .get()
                .unwrap(),
            camera1
        );
    }

    #[test]
    fn update_great_grandchild() {
        let mut app = setup_test_app();
        let world = app.world_mut();

        let scale = 1.;
        let size = UVec2::new(100, 100);

        let camera = world
            .spawn((
                Camera2d,
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size: size,
                            scale_factor: scale,
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .id();

        let uinode = world.spawn(Node::default()).id();
        world.spawn(Node::default()).with_children(|builder| {
            builder.spawn(Node::default()).with_children(|builder| {
                builder.spawn(Node::default()).add_child(uinode);
            });
        });

        app.update();
        let world = app.world_mut();

        assert_eq!(
            world
                .get::<ComputedUiRenderTargetInfo>(uinode)
                .unwrap()
                .scale_factor,
            scale
        );

        assert_eq!(
            world
                .get::<ComputedUiRenderTargetInfo>(uinode)
                .unwrap()
                .physical_size,
            size
        );

        assert_eq!(
            world
                .get::<ComputedUiTargetCamera>(uinode)
                .unwrap()
                .get()
                .unwrap(),
            camera
        );

        world.resource_mut::<UiScale>().0 = 2.;

        app.update();
        let world = app.world_mut();

        assert_eq!(
            world
                .get::<ComputedUiRenderTargetInfo>(uinode)
                .unwrap()
                .scale_factor(),
            2.
        );
    }

    #[test]
    fn fixed_node_opens_new_clipping_context() {
        let mut app = App::new();
        app.add_systems(bevy_app::Update, update_clipping_system);

        let grandchild = app.world_mut().spawn(Node::default()).id();
        let child = app
            .world_mut()
            .spawn(Node::default())
            .add_child(grandchild)
            .id();
        app.world_mut()
            .spawn(Node {
                overflow: Overflow::clip(),
                ..default()
            })
            .add_child(child);

        app.update();
        assert_eq!(
            app.world()
                .get::<CalculatedClip>(grandchild)
                .unwrap()
                .rects()
                .unwrap()
                .len(),
            1
        );

        app.world_mut().entity_mut(child).insert(FixedNode);
        app.update();
        assert!(app.world().get::<CalculatedClip>(grandchild).is_none());

        app.world_mut().entity_mut(child).remove::<FixedNode>();
        app.update();
        assert_eq!(
            app.world()
                .get::<CalculatedClip>(grandchild)
                .unwrap()
                .rects()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn override_clip_opens_new_clipping_context() {
        let mut app = App::new();
        app.add_systems(bevy_app::Update, update_clipping_system);

        let grandchild = app.world_mut().spawn(Node::default()).id();
        let child = app
            .world_mut()
            .spawn((Node::default(), OverrideClip))
            .add_child(grandchild)
            .id();
        app.world_mut()
            .spawn(Node {
                overflow: Overflow::clip(),
                ..default()
            })
            .add_child(child);

        app.update();
        assert!(app.world().get::<CalculatedClip>(grandchild).is_none());
    }
}
