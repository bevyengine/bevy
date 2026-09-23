use crate::layout::{mark_dirty_ui_trees, UiTreeDirty};
use crate::layout_tree::compute_layout;
use crate::layout_tree::TaffyStyle;
use crate::update_computed_nodes;
use crate::UiSystems;
use crate::{clear_transient_dirty_flags, update_border_radius};
use crate::{
    layout::clipping::update_clipping_system, layout::layout_tree::ComputedLayout, prelude::*,
    sync_font_size_to_em_size, sync_taffy_styles_with_nodes, ui_layout_system,
    update::propagate_ui_target_cameras, ContentSize,
};
use bevy_app::{App, HierarchyPropagatePlugin, PostUpdate, PropagateSet, TaskPoolPlugin};
use bevy_camera::{Camera, Camera2d, ComputedCameraValues, RenderTargetInfo, Viewport};
use bevy_ecs::{prelude::*, system::RunSystemOnce};
use bevy_math::{BVec2, Rect, UVec2, Vec2};
use bevy_text::TextFont;
use bevy_utils::prelude::default;

const TARGET_WIDTH: u32 = 1000;
const TARGET_HEIGHT: u32 = 100;

fn setup_ui_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(TaskPoolPlugin::default());

    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
        PostUpdate,
    ));
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(
        PostUpdate,
    ));
    app.init_resource::<UiScale>();
    app.init_resource::<bevy_text::TextPipeline>();
    app.init_resource::<bevy_text::FontCx>();
    app.init_resource::<RemSize>();
    app.init_resource::<bevy_text::ScaleCx>();
    app.init_resource::<bevy_transform::StaticTransformOptimizations>();

    app.add_systems(
        PostUpdate,
        (
            ApplyDeferred,
            propagate_ui_target_cameras,
            clear_transient_dirty_flags,
            sync_font_size_to_em_size,
            sync_taffy_styles_with_nodes,
            mark_dirty_ui_trees,
            ui_layout_system,
            update_computed_nodes,
            update_border_radius,
            update_clipping_system,
        )
            .chain(),
    );

    app.configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiTargetCamera>::default()
            .after(propagate_ui_target_cameras)
            .before(sync_taffy_styles_with_nodes),
    );

    app.configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiRenderTargetInfo>::default()
            .after(propagate_ui_target_cameras)
            .before(sync_taffy_styles_with_nodes),
    );

    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(TARGET_WIDTH, TARGET_HEIGHT),
                    scale_factor: 1.,
                }),
                ..Default::default()
            },
            viewport: Some(Viewport {
                physical_size: UVec2::new(TARGET_WIDTH, TARGET_HEIGHT),
                ..default()
            }),
            ..Default::default()
        },
    ));

    app
}

#[test]
fn ui_nodes_with_percent_100_dimensions_should_fill_their_parent() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();
    let ui_root = world
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        })
        .id();
    let ui_child = world
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        })
        .id();
    world.entity_mut(ui_root).add_child(ui_child);

    app.update();

    for ui_entity in [ui_root, ui_child] {
        let layout = app
            .world()
            .get::<ComputedLayout>(ui_entity)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0;
        assert_eq!(layout.size.width, TARGET_WIDTH as f32);
        assert_eq!(layout.size.height, TARGET_HEIGHT as f32);
    }
}

#[test]
fn computed_layout_lifecycle() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();

    let ui_entity = world.spawn(Node::default()).id();
    assert!(!app
        .world()
        .get::<ComputedLayout>(ui_entity)
        .is_some_and(ComputedLayout::has_layout));

    app.update();
    assert!(app
        .world()
        .get::<ComputedLayout>(ui_entity)
        .is_some_and(ComputedLayout::has_layout));

    app.world_mut().despawn(ui_entity);
    app.update();
    assert!(app.world().get::<ComputedLayout>(ui_entity).is_none());
}

#[test]
fn layouts_are_removed_when_nodes_despawn() {
    let mut app = setup_ui_test_app();
    let entity = app.world_mut().spawn(Node::default()).id();

    app.update();
    assert!(app
        .world()
        .get::<ComputedLayout>(entity)
        .is_some_and(ComputedLayout::has_layout));

    app.world_mut().despawn(entity);
    app.update();

    assert!(!app
        .world()
        .get::<ComputedLayout>(entity)
        .is_some_and(ComputedLayout::has_layout));
}

#[test]
fn node_removal_and_reinsert_should_work() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, (With<Node>, With<ComputedLayout>)>()
            .iter(world)
            .count(),
        0
    );

    let ui_entity = world.spawn(Node::default()).id();

    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, (With<Node>, With<ComputedLayout>)>()
            .single(world)
            .unwrap(),
        ui_entity
    );

    app.world_mut().entity_mut(ui_entity).remove::<Node>();
    app.world_mut().entity_mut(ui_entity).insert(Node {
        width: px(100.),
        ..default()
    });

    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, (With<Node>, With<ComputedLayout>)>()
            .single(world)
            .unwrap(),
        ui_entity
    );
    assert_eq!(
        world
            .get::<ComputedLayout>(ui_entity)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        100.
    );
}

#[test]
fn node_addition_should_sync_children() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    // spawn an invalid UI root node
    let child = world.spawn(Node::default()).id();
    let root = world.spawn(()).add_child(child).id();

    app.update();
    assert!(!app
        .world()
        .get::<ComputedLayout>(child)
        .is_some_and(ComputedLayout::has_layout));

    // fix the invalid root node by inserting a Node
    app.world_mut().entity_mut(root).insert(Node::default());

    app.update();
    // The root node's child should have a layout after update
    assert!(app
        .world()
        .get::<ComputedLayout>(child)
        .is_some_and(ComputedLayout::has_layout));
}

#[test]
fn node_addition_should_sync_parent_and_children() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let d = world.spawn(Node::default()).id();
    let c = world.spawn(()).add_child(d).id();
    let b = world.spawn(Node::default()).id();
    let a = world.spawn(Node::default()).add_children(&[b, c]).id();

    app.update();
    assert!(!app
        .world()
        .get::<ComputedLayout>(d)
        .is_some_and(ComputedLayout::has_layout));

    // fix the invalid middle node by inserting a Node
    app.world_mut().entity_mut(c).insert(Node::default());

    app.update();
    for entity in [a, b, c, d] {
        assert!(app
            .world()
            .get::<ComputedLayout>(entity)
            .is_some_and(ComputedLayout::has_layout));
    }
}

/// regression test for >=0.13.1 root node layouts
/// ensure root nodes act like they are absolutely positioned
/// without explicitly declaring it.
#[test]
fn ui_root_node_should_act_like_position_absolute() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let mut size = 150.;

    world.spawn(Node {
        // test should pass without explicitly requiring position_type to be set to Absolute
        // position_type: PositionType::Absolute,
        width: Val::Px(size),
        height: Val::Px(size),
        ..default()
    });

    size -= 50.;

    world.spawn(Node {
        // position_type: PositionType::Absolute,
        width: Val::Px(size),
        height: Val::Px(size),
        ..default()
    });

    size -= 50.;

    world.spawn(Node {
        // position_type: PositionType::Absolute,
        width: Val::Px(size),
        height: Val::Px(size),
        ..default()
    });

    app.update();
    let world = app.world_mut();

    let overlap_check = world
        .query_filtered::<(Entity, &ComputedNode, &UiGlobalTransform), Without<ChildOf>>()
        .iter(world)
        .fold(
            Option::<(Rect, bool)>::None,
            |option_rect, (entity, node, transform)| {
                let current_rect = Rect::from_center_size(transform.translation, node.size());
                assert!(
                    current_rect.height().abs() + current_rect.width().abs() > 0.,
                    "root ui node {entity} doesn't have a logical size"
                );
                assert_ne!(
                    *transform,
                    UiGlobalTransform::default(),
                    "root ui node {entity} transform is not populated"
                );
                let Some((rect, is_overlapping)) = option_rect else {
                    return Some((current_rect, false));
                };
                if rect.contains(current_rect.center()) {
                    Some((current_rect, true))
                } else {
                    Some((current_rect, is_overlapping))
                }
            },
        );

    let Some((_rect, is_overlapping)) = overlap_check else {
        unreachable!("test not setup properly");
    };
    assert!(is_overlapping, "root ui nodes are expected to behave like they have absolute position and be independent from each other");
}

#[test]
fn ui_node_should_properly_update_when_changing_target_camera() {
    #[derive(Component)]
    struct MovingUiNode;

    fn update_camera_viewports(mut cameras: Query<&mut Camera>) {
        let camera_count = cameras.iter().len();
        for (camera_index, mut camera) in cameras.iter_mut().enumerate() {
            let target_size = camera.physical_target_size().unwrap();
            let viewport_width = target_size.x / camera_count as u32;
            let physical_position = UVec2::new(viewport_width * camera_index as u32, 0);
            let physical_size = UVec2::new(target_size.x / camera_count as u32, target_size.y);
            camera.viewport = Some(Viewport {
                physical_position,
                physical_size,
                ..default()
            });
        }
    }

    fn move_ui_node(
        In(pos): In<Vec2>,
        mut commands: Commands,
        cameras: Query<(Entity, &Camera)>,
        moving_ui_query: Query<Entity, With<MovingUiNode>>,
    ) {
        let (target_camera_entity, _) = cameras
            .iter()
            .find(|(_, camera)| {
                let Some(logical_viewport_rect) = camera.logical_viewport_rect() else {
                    panic!("missing logical viewport")
                };
                // make sure cursor is in viewport and that viewport has at least 1px of size
                logical_viewport_rect.contains(pos)
                    && logical_viewport_rect.max.cmpge(Vec2::splat(0.)).any()
            })
            .expect("cursor position outside of camera viewport");
        for moving_ui_entity in moving_ui_query.iter() {
            commands
                .entity(moving_ui_entity)
                .insert(UiTargetCamera(target_camera_entity))
                .insert(Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(pos.y),
                    left: Val::Px(pos.x),
                    ..default()
                });
        }
    }

    fn do_move_and_test(app: &mut App, new_pos: Vec2, expected_camera_entity: &Entity) {
        let world = app.world_mut();
        world.run_system_once_with(move_ui_node, new_pos).unwrap();
        app.update();
        let world = app.world_mut();
        let (ui_node_entity, UiTargetCamera(target_camera_entity)) = world
            .query_filtered::<(Entity, &UiTargetCamera), With<MovingUiNode>>()
            .single(world)
            .expect("missing MovingUiNode");
        assert_eq!(expected_camera_entity, target_camera_entity);

        let layout = world
            .get::<ComputedLayout>(ui_node_entity)
            .and_then(|layout| layout.get_layout(true))
            .expect("failed to get layout")
            .0;

        // negative test for #12255
        assert_eq!(Vec2::new(layout.location.x, layout.location.y), new_pos);
    }

    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    world.spawn((
        Camera2d,
        Camera {
            order: 1,
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(TARGET_WIDTH, TARGET_HEIGHT),
                    scale_factor: 1.,
                }),
                ..default()
            },
            viewport: Some(Viewport {
                physical_size: UVec2::new(TARGET_WIDTH, TARGET_HEIGHT),
                ..default()
            }),
            ..default()
        },
    ));

    world.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.),
            left: Val::Px(0.),
            ..default()
        },
        MovingUiNode,
    ));

    app.update();
    let world = app.world_mut();

    let pos_inc = Vec2::splat(1.);
    world.run_system_once(update_camera_viewports).unwrap();

    app.update();
    let world = app.world_mut();

    let viewport_rects = world
        .query::<(Entity, &Camera)>()
        .iter(world)
        .map(|(e, c)| (e, c.logical_viewport_rect().expect("missing viewport")))
        .collect::<Vec<_>>();

    for (camera_entity, viewport) in viewport_rects.iter() {
        let target_pos = viewport.min + pos_inc;
        do_move_and_test(&mut app, target_pos, camera_entity);
    }

    // reverse direction
    let mut viewport_rects = viewport_rects.clone();
    viewport_rects.reverse();
    for (camera_entity, viewport) in viewport_rects.iter() {
        let target_pos = viewport.max - pos_inc;
        do_move_and_test(&mut app, target_pos, camera_entity);
    }
}

#[test]
fn compute_layout_uses_camera_viewport() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let root_node_entity = world.spawn(Node::default()).id();

    fn test_system(
        In(root_node_entity): In<Entity>,
        ui_children: Query<(Option<&Children>, Has<GhostNode>, Ref<UiTreeDirty>), With<Node>>,
        node_query: Query<
            (
                Ref<TaffyStyle>,
                Ref<ContentSize>,
                Has<FixedNode>,
                Ref<UiTransform>,
                Ref<ScrollPosition>,
                Option<Ref<Outline>>,
                Option<Ref<LayoutConfig>>,
                Option<Ref<IgnoreScroll>>,
                Has<OverrideClip>,
                Ref<UiTreeDirty>,
            ),
            With<Node>,
        >,
        style_query: Query<&'static TaffyStyle>,
        mut node_queries: ParamSet<(Query<&mut ComputedLayout>,)>,
        mut buffer_query: Query<&mut bevy_text::ComputedTextBlock>,
        mut font_system: ResMut<bevy_text::FontCx>,
        mut child_stack: Local<Vec<taffy::NodeId>>,
        mut ghost_stack: Local<Vec<Entity>>,
    ) {
        compute_layout(
            root_node_entity,
            UVec2::new(800, 600),
            &ui_children,
            &node_query,
            &style_query,
            &mut node_queries.p0(),
            &[],
            &mut buffer_query,
            &mut font_system,
            &mut child_stack,
            true,
            &mut ghost_stack,
        )
        .unwrap();
    }

    world
        .run_system_once_with(test_system, root_node_entity)
        .unwrap();

    assert!(world
        .get::<ComputedLayout>(root_node_entity)
        .is_some_and(ComputedLayout::has_layout));
}

#[test]
fn fixed_root_is_a_root_node() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let fixed_entity = world
        .spawn((
            Node {
                width: Val::Percent(50.),
                height: Val::Percent(50.),
                ..default()
            },
            FixedNode,
        ))
        .id();

    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .get::<ComputedLayout>(fixed_entity)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.5
    );
}

#[test]
fn swap_fixed_nodes() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let a = world
        .spawn(Node {
            width: Val::Percent(20.),
            height: Val::Percent(20.),
            ..default()
        })
        .id();
    let b = world
        .spawn((
            Node {
                width: Val::Percent(50.),
                height: Val::Percent(50.),
                ..default()
            },
            ChildOf(a),
        ))
        .id();

    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .get::<ComputedLayout>(a)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.2
    );
    assert_eq!(
        world
            .get::<ComputedLayout>(b)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.2 * 0.5
    );

    world.entity_mut(a).insert(FixedNode);

    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .get::<ComputedLayout>(b)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.2 * 0.5
    );

    world.entity_mut(b).insert(FixedNode);

    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .get::<ComputedLayout>(b)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.5
    );

    world.entity_mut(b).remove::<ChildOf>().add_child(a);

    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .get::<ComputedLayout>(a)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.2
    );
    assert_eq!(
        world
            .get::<ComputedLayout>(b)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.5
    );

    world.entity_mut(b).remove::<FixedNode>();

    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .get::<ComputedLayout>(a)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.2
    );

    world.entity_mut(a).remove::<FixedNode>();

    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .get::<ComputedLayout>(a)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.5 * 0.2
    );
}

#[test]
fn fixed_node_children() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let a = world
        .spawn(Node {
            width: Val::Percent(50.),
            height: Val::Percent(50.),
            ..default()
        })
        .id();
    let b = world
        .spawn(Node {
            width: Val::Percent(50.),
            height: Val::Percent(50.),
            ..default()
        })
        .id();
    let c = world
        .spawn(Node {
            width: Val::Percent(50.),
            height: Val::Percent(50.),
            ..default()
        })
        .id();
    let p = world
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            width: Val::Px(200.),
            height: Val::Px(100.),
            ..default()
        })
        .add_children(&[a, b, c])
        .id();

    app.update();
    let world = app.world_mut();
    for entity in [a, b, c] {
        assert_eq!(
            world
                .get::<ComputedLayout>(entity)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            100.
        );
    }

    world.entity_mut(a).insert(FixedNode);

    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .get::<ComputedLayout>(a)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.5
    );
    for entity in [b, c] {
        assert_eq!(
            world
                .get::<ComputedLayout>(entity)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            100.
        );
    }

    world.entity_mut(c).insert(FixedNode);
    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .get::<ComputedLayout>(b)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        100.
    );
    for entity in [a, c] {
        assert_eq!(
            world
                .get::<ComputedLayout>(entity)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.5
        );
    }

    world.entity_mut(p).detach_all_children();
    world.entity_mut(p).despawn();

    app.update();
    let world = app.world_mut();
    for entity in [a, b, c] {
        assert_eq!(
            world
                .get::<ComputedLayout>(entity)
                .and_then(|layout| layout.get_layout(true))
                .unwrap()
                .0
                .size
                .width,
            TARGET_WIDTH as f32 * 0.5
        );
    }
}

#[test]
fn reparenting_recomputes_from_current_entity_tree() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let root_a = world
        .spawn(Node {
            width: px(100.),
            height: px(20.),
            ..default()
        })
        .id();
    let root_b = world
        .spawn(Node {
            width: px(200.),
            height: px(20.),
            ..default()
        })
        .id();
    let child = world
        .spawn(Node {
            width: percent(100.),
            height: px(10.),
            ..default()
        })
        .id();

    world.entity_mut(root_a).add_child(child);
    app.update();
    assert_eq!(
        app.world()
            .get::<ComputedLayout>(child)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        100.
    );

    let world = app.world_mut();
    world.entity_mut(root_a).detach_child(child);
    world.entity_mut(root_b).add_child(child);
    app.update();
    assert_eq!(
        app.world()
            .get::<ComputedLayout>(child)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        200.
    );
}

#[test]
fn child_style_change_invalidates_parent_cache() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let child = world
        .spawn(Node {
            width: px(50.),
            height: px(10.),
            ..default()
        })
        .id();
    let root = world
        .spawn(Node {
            width: px(100.),
            height: px(20.),
            ..default()
        })
        .add_child(child)
        .id();

    app.update();
    app.update();

    app.world_mut()
        .entity_mut(child)
        .get_mut::<Node>()
        .unwrap()
        .width = px(75.);

    app.update();

    assert_eq!(
        app.world()
            .get::<ComputedLayout>(root)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        100.
    );
    assert_eq!(
        app.world()
            .get::<ComputedLayout>(child)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        75.
    );
}

#[test]
fn ui_node_should_be_set_to_its_content_size() {
    let mut app = setup_ui_test_app();
    let content_size = Vec2::new(50., 25.);

    let ui_entity = app
        .world_mut()
        .spawn((
            Node {
                align_self: AlignSelf::Start,
                ..default()
            },
            ContentSize::fixed_size(content_size),
        ))
        .id();

    app.update();
    let layout = app
        .world()
        .get::<ComputedLayout>(ui_entity)
        .and_then(|layout| layout.get_layout(true))
        .unwrap()
        .0;

    assert_eq!(layout.size.width, content_size.x);
    assert_eq!(layout.size.height, content_size.y);
}

#[test]
fn measured_node_includes_border_and_padding() {
    let mut app = setup_ui_test_app();

    let ui_node = app
        .world_mut()
        .spawn((
            Node {
                align_self: AlignSelf::Start,
                border: UiRect {
                    left: px(2.0),
                    right: px(6.0),
                    top: px(4.0),
                    bottom: px(8.0),
                },
                padding: UiRect {
                    left: px(3.0),
                    right: px(5.0),
                    top: px(7.0),
                    bottom: px(11.0),
                },
                ..default()
            },
            ContentSize::fixed_size(Vec2::new(50.0, 25.0)),
        ))
        .id();

    app.update();
    let layout = app
        .world()
        .get::<ComputedLayout>(ui_node)
        .and_then(|layout| layout.get_layout(true))
        .unwrap()
        .0;

    assert_eq!(layout.border.left, 2.0);
    assert_eq!(layout.border.right, 6.0);
    assert_eq!(layout.border.top, 4.0);
    assert_eq!(layout.border.bottom, 8.0);
    assert_eq!(layout.padding.left, 3.0);
    assert_eq!(layout.padding.right, 5.0);
    assert_eq!(layout.padding.top, 7.0);
    assert_eq!(layout.padding.bottom, 11.0);
    assert_eq!(layout.size.width, 66.0);
    assert_eq!(layout.size.height, 55.0);
    assert_eq!(
        app.world()
            .get::<ComputedNode>(ui_node)
            .unwrap()
            .padding_box()
            .size(),
        Vec2::new(58.0, 43.0)
    );
    assert_eq!(layout.content_box_width(), 50.0);
    assert_eq!(layout.content_box_height(), 25.0);
}

#[test]
fn measure_funcs_persist_until_cleared() {
    let mut app = setup_ui_test_app();
    let content_size = Vec2::new(50., 25.);
    let ui_entity = app
        .world_mut()
        .spawn((Node::default(), ContentSize::fixed_size(content_size)))
        .id();

    app.update();
    let layout = app
        .world()
        .get::<ComputedLayout>(ui_entity)
        .and_then(|layout| layout.get_layout(true))
        .unwrap()
        .0;
    assert_eq!(layout.size.width, content_size.x);
    assert_eq!(layout.size.height, content_size.y);

    app.update();
    let layout = app
        .world()
        .get::<ComputedLayout>(ui_entity)
        .and_then(|layout| layout.get_layout(true))
        .unwrap()
        .0;
    assert_eq!(layout.size.width, content_size.x);
    assert_eq!(layout.size.height, content_size.y);

    app.world_mut()
        .entity_mut(ui_entity)
        .get_mut::<ContentSize>()
        .unwrap()
        .clear();

    app.update();
    let layout = app
        .world()
        .get::<ComputedLayout>(ui_entity)
        .and_then(|layout| layout.get_layout(true))
        .unwrap()
        .0;
    assert_eq!(layout.size.width, 0.);
    assert_eq!(layout.size.height, 0.);
}

#[test]
fn get_layout_can_return_unrounded_layout() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let root = world
        .spawn(Node {
            width: px(101.),
            height: px(10.),
            ..default()
        })
        .id();
    let child = world
        .spawn(Node {
            width: percent(50.),
            height: px(10.),
            ..default()
        })
        .id();
    world.entity_mut(root).add_child(child);

    app.update();

    let rounded = app
        .world()
        .get::<ComputedLayout>(child)
        .and_then(|layout| layout.get_layout(true))
        .unwrap()
        .0;
    let unrounded = app
        .world()
        .get::<ComputedLayout>(child)
        .and_then(|layout| layout.get_layout(false))
        .unwrap()
        .0;
    assert_eq!(unrounded.size.width, 50.5);
    assert_ne!(rounded.size.width, unrounded.size.width);
}

#[test]
fn fixed_child_uses_viewport_layout_context() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let parent = world
        .spawn(Node {
            width: px(200.),
            height: px(20.),
            ..default()
        })
        .id();
    let fixed = world
        .spawn((
            Node {
                width: percent(50.),
                height: px(10.),
                ..default()
            },
            FixedNode,
            ChildOf(parent),
        ))
        .id();

    app.update();

    assert_eq!(
        app.world()
            .get::<ComputedLayout>(parent)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        200.
    );
    assert_eq!(
        app.world()
            .get::<ComputedLayout>(fixed)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.5
    );
}

#[test]
fn fixed_node_changes_recompute_parent_and_child_layouts() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let parent = world
        .spawn(Node {
            width: px(200.),
            height: px(20.),
            ..default()
        })
        .id();
    let child = world
        .spawn((
            Node {
                width: percent(50.),
                height: px(10.),
                ..default()
            },
            FixedNode,
            ChildOf(parent),
        ))
        .id();

    app.update();
    assert_eq!(
        app.world()
            .get::<ComputedLayout>(child)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.5
    );

    app.world_mut().entity_mut(child).remove::<FixedNode>();
    app.update();
    assert_eq!(
        app.world()
            .get::<ComputedLayout>(child)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        100.
    );

    app.world_mut().entity_mut(child).insert(FixedNode);
    app.update();
    assert_eq!(
        app.world()
            .get::<ComputedLayout>(child)
            .and_then(|layout| layout.get_layout(true))
            .unwrap()
            .0
            .size
            .width,
        TARGET_WIDTH as f32 * 0.5
    );
}

#[test]
fn ui_rounding_test() {
    let mut app = setup_ui_test_app();
    let parent = app
        .world_mut()
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::min_content(2),
            margin: UiRect::all(Val::Px(4.0)),
            ..default()
        })
        .with_children(|commands| {
            for _ in 0..2 {
                commands.spawn(Node {
                    display: Display::Grid,
                    width: Val::Px(160.),
                    height: Val::Px(160.),
                    ..default()
                });
            }
        })
        .id();

    let children = app
        .world()
        .entity(parent)
        .get::<Children>()
        .unwrap()
        .iter()
        .collect::<Vec<Entity>>();

    for r in [2, 3, 5, 7, 11, 13, 17, 19, 21, 23, 29, 31].map(|n| (n as f32).recip()) {
        let mut s = 1. - r;
        while s <= 5. {
            app.world_mut().resource_mut::<UiScale>().0 = s;
            app.update();
            let world = app.world();
            let width_sum: f32 = children
                .iter()
                .map(|child| world.get::<ComputedNode>(*child).unwrap().size.x)
                .sum();
            let parent_width = world.get::<ComputedNode>(parent).unwrap().size.x;
            assert!((width_sum - parent_width).abs() < 0.001);
            assert!((width_sum - 320. * s).abs() <= 1.);
            s += r;
        }
    }
}

#[test]
fn no_camera_ui() {
    let mut app = App::new();

    app.add_systems(
        PostUpdate,
        (propagate_ui_target_cameras, ApplyDeferred)
            .chain()
            .before(UiSystems::Layout),
    );

    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
        PostUpdate,
    ));

    app.configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiTargetCamera>::default()
            .after(propagate_ui_target_cameras)
            .before(UiSystems::Layout),
    );

    let world = app.world_mut();
    world.init_resource::<UiScale>();
    world.init_resource::<bevy_text::TextPipeline>();
    world.init_resource::<bevy_text::FontCx>();
    world.init_resource::<RemSize>();
    world.init_resource::<bevy_text::ScaleCx>();

    let ui_root = world
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        })
        .id();
    let ui_child = world
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        })
        .id();
    world.entity_mut(ui_root).add_child(ui_child);

    app.update();
}

#[test]
fn rem_sized_node_is_rem_sized() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();
    let ui_root = world
        .spawn(Node {
            width: Val::Rem(3.),
            height: Val::Rem(2.),
            ..default()
        })
        .id();
    world.insert_resource(UiScale(5.));

    app.update();

    let world = app.world_mut();
    let c = world.entity(ui_root).get::<ComputedNode>().unwrap();
    assert!(c.size().abs_diff_eq(
        world.resource::<RemSize>().0 * world.resource::<UiScale>().0 * Vec2::new(3., 2.),
        1e-5
    ));

    world.insert_resource(RemSize(100.));

    app.update();

    let world = app.world_mut();
    let c = world.entity(ui_root).get::<ComputedNode>().unwrap();
    assert!(c.size().abs_diff_eq(
        world.resource::<RemSize>().0 * world.resource::<UiScale>().0 * Vec2::new(3., 2.),
        1e-5
    ));
}

#[test]
fn em_and_rem_sized_nodes_are_updated_on_changes_to_em_and_rem_sizes() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();

    let ui_root = world
        .spawn((
            Node {
                width: Val::Rem(20.),
                height: Val::Em(30.),
                ..default()
            },
            TextFont::default().with_font_size(5.),
        ))
        .id();

    let child = world
        .spawn((
            Node {
                width: Val::Em(5.),
                height: Val::Rem(4.),
                ..default()
            },
            TextFont::default().with_font_size(15.),
            ChildOf(ui_root),
        ))
        .id();

    app.update();

    let world = app.world_mut();

    world.resource_mut::<RemSize>().0 = 10.;

    app.update();
    let world = app.world_mut();

    let computed_root = world.entity(ui_root).get::<ComputedNode>().unwrap();

    assert!(computed_root
        .size()
        .abs_diff_eq(Vec2::new(200., 150.), 1e-5));
    let computed_child = world.entity(child).get::<ComputedNode>().unwrap();
    assert!(computed_child.size().abs_diff_eq(Vec2::new(75., 40.), 1e-5));
}

#[test]
fn removing_node_from_ui_child_should_relayout_parent() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();
    let ui_root = world.spawn(Node::default()).id();
    let ui_child = world
        .spawn((
            Node {
                width: px(50.),
                height: px(30.),
                ..default()
            },
            ChildOf(ui_root),
        ))
        .id();

    app.update();

    let world = app.world_mut();
    world.entity_mut(ui_child).remove::<Node>();

    app.update();

    let world = app.world_mut();
    assert!(world
        .entity(ui_root)
        .get::<ComputedNode>()
        .unwrap()
        .size()
        .abs_diff_eq(Vec2::ZERO, 1e-5));
}

#[test]
fn block_layouts_margins_collapse() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let a = world
        .spawn(Node {
            height: px(50),
            margin: px(100).bottom(),
            ..default()
        })
        .id();
    let b = world
        .spawn(Node {
            height: px(50),
            margin: px(50).top(),
            ..default()
        })
        .id();
    world
        .spawn(Node {
            display: Display::Block,
            ..default()
        })
        .add_children(&[a, b]);

    app.update();

    let world = app.world();
    let computed_a = world.get::<ComputedNode>(a).unwrap();
    let transform_a = world.get::<UiGlobalTransform>(a).unwrap();
    let computed_b = world.get::<ComputedNode>(b).unwrap();
    let transform_b = world.get::<UiGlobalTransform>(b).unwrap();
    let a_bottom = 0.5 * computed_a.size.y + transform_a.affine().translation.y;
    let b_top = -0.5 * computed_b.size.y + transform_b.affine().translation.y;
    assert!((b_top - a_bottom - 100.).abs() <= 1e-5);
}

#[test]
fn block_layouts_nested_margins_collapse() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let a = world
        .spawn(Node {
            height: px(50),
            ..default()
        })
        .id();
    let nested_child = world
        .spawn(Node {
            display: Display::Block,
            margin: UiRect::vertical(px(40)),
            ..default()
        })
        .id();
    let nested = world
        .spawn(Node {
            display: Display::Block,
            ..default()
        })
        .add_child(nested_child)
        .id();
    let b = world
        .spawn(Node {
            height: px(50),
            ..default()
        })
        .id();
    world
        .spawn(Node {
            display: Display::Block,
            ..default()
        })
        .add_children(&[a, nested, b]);

    app.update();

    let world = app.world();
    let computed_a = world.get::<ComputedNode>(a).unwrap();
    let transform_a = world.get::<UiGlobalTransform>(a).unwrap();
    let computed_b = world.get::<ComputedNode>(b).unwrap();
    let transform_b = world.get::<UiGlobalTransform>(b).unwrap();
    let a_bottom = 0.5 * computed_a.size.y + transform_a.affine().translation.y;
    let b_top = -0.5 * computed_b.size.y + transform_b.affine().translation.y;
    assert!((b_top - a_bottom - 40.).abs() <= 1e-5);
}

#[test]
fn block_layouts_respect_align_content() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let child = world
        .spawn(Node {
            height: px(20),
            ..default()
        })
        .id();
    world
        .spawn(Node {
            display: Display::Block,
            align_content: AlignContent::End,
            height: px(100),
            ..default()
        })
        .add_child(child);

    app.update();

    assert_eq!(
        app.world()
            .get::<UiGlobalTransform>(child)
            .map(|transform| transform.translation.y),
        Some(90.)
    );
}

#[test]
fn test_border_radius_updates() {
    let mut app = setup_ui_test_app();

    let entity = app
        .world_mut()
        .spawn((Node {
            height: px(100),
            width: px(50),
            ..default()
        },))
        .id();

    app.update();

    let computed = app.world().get::<ComputedNode>(entity).unwrap();
    assert_eq!(computed.border_radius, ResolvedBorderRadius::ZERO);

    app.world_mut()
        .get_mut::<Node>(entity)
        .unwrap()
        .border_radius = BorderRadius::all(px(10));

    app.update();

    let computed = app.world().get::<ComputedNode>(entity).unwrap();
    assert_eq!(
        computed.border_radius,
        ResolvedBorderRadius {
            top_left: Vec2::splat(10.),
            top_right: Vec2::splat(10.),
            bottom_left: Vec2::splat(10.),
            bottom_right: Vec2::splat(10.)
        }
    );

    app.world_mut()
        .get_mut::<Node>(entity)
        .unwrap()
        .border_radius
        .top_left = CornerRadius::circular(vh(30));

    app.update();

    assert_eq!(
        app.world()
            .get::<ComputedNode>(entity)
            .unwrap()
            .border_radius,
        ResolvedBorderRadius {
            top_left: Vec2::splat(TARGET_HEIGHT as f32 * 30. / 100.).min(Vec2::splat(25.)),
            top_right: Vec2::splat(10.),
            bottom_left: Vec2::splat(10.),
            bottom_right: Vec2::splat(10.)
        }
    );

    let border_radius = &mut app
        .world_mut()
        .get_mut::<Node>(entity)
        .unwrap()
        .border_radius;
    border_radius.top_right = CornerRadius::circular(percent(100));
    border_radius.bottom_left = CornerRadius::new(percent(100), percent(100));

    app.update();

    assert_eq!(
        app.world()
            .get::<ComputedNode>(entity)
            .unwrap()
            .border_radius,
        ResolvedBorderRadius {
            top_left: Vec2::splat(TARGET_HEIGHT as f32 * 30. / 100.).min(Vec2::splat(25.)),
            top_right: Vec2::splat(25.),
            bottom_left: Vec2::new(25., 50.),
            bottom_right: Vec2::splat(10.)
        }
    );

    app.world_mut().get_mut::<Node>(entity).unwrap().width = px(200.);

    app.update();

    assert_eq!(
        app.world()
            .get::<ComputedNode>(entity)
            .unwrap()
            .border_radius,
        ResolvedBorderRadius {
            top_left: Vec2::splat(TARGET_HEIGHT as f32 * 30. / 100.).min(Vec2::splat(50.)),
            top_right: Vec2::splat(50.),
            bottom_left: Vec2::new(100., 50.),
            bottom_right: Vec2::splat(10.)
        }
    );

    let world = app.world_mut();
    let mut camera_query = world.query::<&mut Camera>();
    camera_query
        .single_mut(world)
        .unwrap()
        .viewport
        .as_mut()
        .unwrap()
        .physical_size
        .y = TARGET_HEIGHT / 2;

    app.update();

    assert_eq!(
        app.world()
            .get::<ComputedNode>(entity)
            .unwrap()
            .border_radius
            .top_left,
        Vec2::splat(15.)
    );
}

#[test]
fn move_child_by_parent_scroll_position() {
    let mut app = setup_ui_test_app();

    let parent = app
        .world_mut()
        .spawn((Node {
            width: px(100),
            height: px(100),
            overflow: Overflow::scroll(),
            ..default()
        },))
        .id();

    let child = app
        .world_mut()
        .spawn((
            Node {
                min_width: px(200.),
                min_height: px(200.),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();

    app.update();

    app.world_mut().get_mut::<ScrollPosition>(parent).unwrap().0 = Vec2::new(50., 100.);

    app.update();

    assert_eq!(
        Vec2::new(50., 0.),
        app.world()
            .get::<UiGlobalTransform>(child)
            .unwrap()
            .translation
    );
}

#[test]
fn move_node_with_uitransform() {
    let mut app = setup_ui_test_app();

    let parent = app
        .world_mut()
        .spawn((Node {
            width: px(100),
            height: px(100),
            ..default()
        },))
        .id();

    let child = app
        .world_mut()
        .spawn((
            Node {
                width: px(100),
                height: px(100),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();

    let grand_child = app
        .world_mut()
        .spawn((
            Node {
                width: px(100),
                height: px(100.),
                ..default()
            },
            ChildOf(child),
        ))
        .id();

    app.update();

    app.world_mut()
        .get_mut::<UiTransform>(parent)
        .unwrap()
        .translation = Val2::px(60., 40.);

    app.update();

    assert_eq!(
        Vec2::new(110., 90.),
        app.world()
            .get::<UiGlobalTransform>(grand_child)
            .unwrap()
            .translation
    );

    app.world_mut()
        .get_mut::<UiTransform>(grand_child)
        .unwrap()
        .translation = Val2::px(20., 30.);

    app.update();

    assert_eq!(
        Vec2::new(130., 120.),
        app.world()
            .get::<UiGlobalTransform>(grand_child)
            .unwrap()
            .translation
    );
}

#[test]
fn fixed_node_doesnt_propagate_parents_uitransform() {
    let mut app = setup_ui_test_app();

    let parent = app
        .world_mut()
        .spawn((
            Node {
                width: px(100),
                height: px(100),
                ..default()
            },
            UiTransform::from_translation(px(50.).into()),
        ))
        .id();

    let child = app
        .world_mut()
        .spawn((
            Node {
                min_width: px(100),
                min_height: px(100),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();

    let grand_child = app
        .world_mut()
        .spawn((
            Node {
                width: px(100),
                height: px(100.),
                ..default()
            },
            ChildOf(child),
        ))
        .id();

    app.update();

    assert_eq!(
        Vec2::new(100., 100.),
        app.world()
            .get::<UiGlobalTransform>(child)
            .unwrap()
            .translation
    );

    assert_eq!(
        Vec2::new(100., 100.),
        app.world()
            .get::<UiGlobalTransform>(grand_child)
            .unwrap()
            .translation
    );

    app.world_mut().entity_mut(child).insert(FixedNode);

    app.update();

    assert_eq!(
        Vec2::new(50., 50.),
        app.world()
            .get::<UiGlobalTransform>(child)
            .unwrap()
            .translation
    );

    assert_eq!(
        Vec2::new(50., 50.),
        app.world()
            .get::<UiGlobalTransform>(grand_child)
            .unwrap()
            .translation
    );

    app.world_mut()
        .get_mut::<UiTransform>(parent)
        .unwrap()
        .translation = Val2::px(10., 10.);

    app.update();

    assert_eq!(
        Vec2::new(50., 50.),
        app.world()
            .get::<UiGlobalTransform>(child)
            .unwrap()
            .translation
    );

    assert_eq!(
        Vec2::new(50., 50.),
        app.world()
            .get::<UiGlobalTransform>(grand_child)
            .unwrap()
            .translation
    );

    app.world_mut().entity_mut(child).remove::<FixedNode>();

    app.update();

    assert_eq!(
        Vec2::new(60., 60.),
        app.world()
            .get::<UiGlobalTransform>(child)
            .unwrap()
            .translation
    );

    assert_eq!(
        Vec2::new(60., 60.),
        app.world()
            .get::<UiGlobalTransform>(grand_child)
            .unwrap()
            .translation
    );
}

#[test]
fn clipping_updates_on_layout_changes() {
    let mut app = setup_ui_test_app();

    let child = app.world_mut().spawn(Node::default()).id();
    let parent = app
        .world_mut()
        .spawn((Node {
            width: Val::Px(60.),
            height: Val::Px(20.),
            overflow: Overflow::clip(),
            ..default()
        },))
        .add_child(child)
        .id();

    app.update();

    let initial_clip = app.world().get::<CalculatedClip>(child).unwrap().clone();

    app.world_mut().get_mut::<Node>(parent).unwrap().width = Val::Px(80.);
    app.update();

    assert_ne!(
        &initial_clip,
        app.world().get::<CalculatedClip>(child).unwrap()
    );
}

#[test]
fn fixed_node_opens_new_clipping_context() {
    let mut app = setup_ui_test_app();

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
    let mut app = setup_ui_test_app();

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

#[test]
fn ghost_nodes_flatten_layout_children() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let child = world
        .spawn(Node {
            width: px(50.),
            height: px(10.),
            ..default()
        })
        .id();
    let mid = world.spawn(GhostNode).add_child(child).id();
    let root = world
        .spawn(Node {
            width: px(100.),
            height: px(20.),
            ..default()
        })
        .add_child(mid)
        .id();

    app.update();
    assert!(app
        .world()
        .get::<ComputedLayout>(child)
        .is_some_and(ComputedLayout::has_layout));

    app.world_mut().entity_mut(mid).remove::<GhostNode>();
    app.update();
    assert!(app
        .world()
        .get::<ComputedLayout>(mid)
        .is_some_and(ComputedLayout::has_layout));

    app.world_mut().entity_mut(mid).insert(GhostNode);
    app.update();
    let root_layout = app
        .world()
        .get::<ComputedLayout>(root)
        .and_then(|layout| layout.get_layout(true))
        .unwrap()
        .0;
    let child_layout = app
        .world()
        .get::<ComputedLayout>(child)
        .and_then(|layout| layout.get_layout(true))
        .unwrap()
        .0;
    assert_eq!(root_layout.size.width, 100.);
    assert_eq!(child_layout.size.width, 50.);
}

#[test]
fn outlines_relayout_on_outline_remove_and_insert() {
    let mut app = setup_ui_test_app();

    let entity = app
        .world_mut()
        .spawn((
            Node::default(),
            Outline {
                width: px(10.),
                offset: px(5.),
                ..default()
            },
        ))
        .id();

    app.update();

    let computed_node = app.world().get::<ComputedNode>(entity).unwrap();
    assert_eq!(computed_node.outline_width(), 10.);
    assert_eq!(computed_node.outline_offset(), 5.);

    app.world_mut().entity_mut(entity).remove::<Outline>();
    app.update();

    let computed_node = app.world().get::<ComputedNode>(entity).unwrap();
    assert_eq!(computed_node.outline_width(), 0.);
    assert_eq!(computed_node.outline_offset(), 0.);

    app.world_mut().entity_mut(entity).insert(Outline {
        width: px(20.),
        offset: px(10.),
        ..default()
    });
    app.update();

    let computed_node = app.world().get::<ComputedNode>(entity).unwrap();
    assert_eq!(computed_node.outline_width(), 20.);
    assert_eq!(computed_node.outline_offset(), 10.);
}

#[test]
fn ignore_scroll_relayouts_on_remove_and_insert() {
    let mut app = setup_ui_test_app();

    let parent = app
        .world_mut()
        .spawn((
            Node {
                width: px(100.),
                height: px(100.),
                overflow: Overflow::scroll_x(),
                ..default()
            },
            ScrollPosition(Vec2::new(20., 0.)),
        ))
        .id();
    let child = app
        .world_mut()
        .spawn((
            Node {
                width: px(200.),
                height: px(100.),
                flex_shrink: 0.,
                ..default()
            },
            IgnoreScroll(BVec2::new(true, false)),
            ChildOf(parent),
        ))
        .id();

    app.update();

    let initial_x = app
        .world()
        .get::<UiGlobalTransform>(child)
        .unwrap()
        .translation
        .x;

    app.world_mut().entity_mut(child).remove::<IgnoreScroll>();
    app.update();

    assert_eq!(
        app.world()
            .get::<UiGlobalTransform>(child)
            .unwrap()
            .translation
            .x,
        initial_x - 20.
    );

    app.world_mut()
        .entity_mut(child)
        .insert(IgnoreScroll(BVec2::new(true, false)));
    app.update();

    assert_eq!(
        app.world()
            .get::<UiGlobalTransform>(child)
            .unwrap()
            .translation
            .x,
        initial_x
    );
}

#[test]
fn layout_config_relayouts_on_remove_and_insert() {
    let mut app = setup_ui_test_app();

    let entity = app
        .world_mut()
        .spawn((
            Node {
                width: px(10.5),
                height: px(10.5),
                ..default()
            },
            LayoutConfig {
                use_rounding: false,
            },
        ))
        .id();

    app.update();

    assert_eq!(
        app.world().get::<ComputedNode>(entity).unwrap().size(),
        Vec2::splat(10.5)
    );

    app.world_mut().entity_mut(entity).remove::<LayoutConfig>();
    app.update();

    assert_eq!(
        app.world().get::<ComputedNode>(entity).unwrap().size(),
        Vec2::splat(11.)
    );

    app.world_mut().entity_mut(entity).insert(LayoutConfig {
        use_rounding: false,
    });
    app.update();

    assert_eq!(
        app.world().get::<ComputedNode>(entity).unwrap().size(),
        Vec2::splat(10.5)
    );
}

#[test]
fn unparenting_ghost_child_makes_child_layout_root() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let child = world.spawn(Node::default()).id();
    let ghost = world.spawn(GhostNode).add_child(child).id();
    let root = world.spawn(Node::default()).add_child(ghost).id();

    app.update();

    let computed_root = app.world().get::<ComputedLayout>(root).unwrap();
    assert!(computed_root.child_entities().eq(core::iter::once(child)));
    assert!(computed_root.has_layout());
    assert!(computed_root.is_layout_root());

    let computed_child = app.world().get::<ComputedLayout>(child).unwrap();
    assert!(computed_child.child_nodes().is_empty());
    assert!(computed_child.has_layout());
    assert!(!computed_child.is_layout_root());

    app.world_mut().entity_mut(ghost).detach_all_children();
    app.update();

    let computed_root = app.world().get::<ComputedLayout>(root).unwrap();
    assert!(computed_root.child_nodes().is_empty());
    assert!(computed_root.has_layout());
    assert!(computed_root.is_layout_root());

    let computed_child = app.world().get::<ComputedLayout>(child).unwrap();
    assert!(computed_root.child_nodes().is_empty());
    assert!(computed_child.has_layout());
    assert!(computed_child.is_layout_root());
}

#[test]
fn despawning_intermediate_ghost_child_makes_child_layout_root() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let child = world.spawn(Node::default()).id();
    let ghost = world.spawn(GhostNode).add_child(child).id();
    let root = world.spawn(Node::default()).add_child(ghost).id();

    app.update();

    let mut ghost_mut = app.world_mut().entity_mut(ghost);
    ghost_mut.detach_all_children();
    ghost_mut.despawn();

    app.update();

    let computed_root = app.world().get::<ComputedLayout>(root).unwrap();
    assert!(computed_root.child_nodes().is_empty());
    assert!(computed_root.has_layout());
    assert!(computed_root.is_layout_root());

    let computed_child = app.world().get::<ComputedLayout>(child).unwrap();
    assert!(computed_child.child_nodes().is_empty());
    assert!(computed_child.has_layout());
    assert!(computed_child.is_layout_root());
}

#[test]
fn adding_intermediate_ghost_node_includes_child_in_layout() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let child = world.spawn(Node::default()).id();
    let mid = world.spawn_empty().add_child(child).id();
    let root = world.spawn(Node::default()).add_child(mid).id();

    app.update();

    let computed_root = app.world().get::<ComputedLayout>(root).unwrap();
    assert!(computed_root.is_layout_root());
    assert!(computed_root.child_nodes().is_empty());

    let computed_child = app.world().get::<ComputedLayout>(child).unwrap();
    assert!(!computed_child.has_layout());

    app.world_mut().entity_mut(mid).insert(GhostNode);

    app.update();

    let computed_root = app.world().get::<ComputedLayout>(root).unwrap();
    assert!(computed_root.child_entities().eq(core::iter::once(child)));

    let computed_child = app.world().get::<ComputedLayout>(child).unwrap();
    assert!(computed_child.has_layout());
}

#[test]
fn removing_intermediate_ghost_node_excludes_child_from_layout() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let child = world.spawn(Node::default()).id();
    let mid = world.spawn(GhostNode).add_child(child).id();
    let root = world.spawn(Node::default()).add_child(mid).id();

    app.update();

    let computed_root = app.world().get::<ComputedLayout>(root).unwrap();
    assert!(computed_root.child_entities().eq(core::iter::once(child)));

    app.world_mut()
        .entity_mut(mid)
        .remove::<(GhostNode, Node)>();

    app.update();

    let computed_root = app.world().get::<ComputedLayout>(root).unwrap();
    assert!(computed_root.child_nodes().is_empty());

    let computed_child = app.world().get::<ComputedLayout>(child).unwrap();
    assert!(!computed_child.has_layout());
}

#[test]
fn fixed_child_of_ghost_is_separate_layout_root() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let fixed = world.spawn((Node::default(), FixedNode)).id();
    let child = world.spawn(Node::default()).id();
    let ghost = world.spawn(GhostNode).add_children(&[fixed, child]).id();
    let root = world.spawn(Node::default()).add_child(ghost).id();

    app.update();
    let computed_root = app.world().get::<ComputedLayout>(root).unwrap();
    assert!(computed_root.child_entities().eq(core::iter::once(child)));

    let computed_ghost = app.world().get::<ComputedLayout>(ghost).unwrap();
    assert!(computed_ghost.child_nodes().is_empty());
    assert!(!computed_ghost.has_layout());

    let computed_fixed = app.world().get::<ComputedLayout>(fixed).unwrap();
    assert!(computed_fixed.has_layout());
    assert!(computed_fixed.is_layout_root());

    let computed_child = app.world().get::<ComputedLayout>(child).unwrap();
    assert!(computed_child.has_layout());
    assert!(!computed_child.is_layout_root());
}

#[test]
fn unghost_ghost_node_with_fixed_and_normal_children() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let fixed = world.spawn((Node::default(), FixedNode)).id();
    let child = world.spawn(Node::default()).id();
    let ghost = world.spawn(GhostNode).add_children(&[fixed, child]).id();

    app.update();

    let computed_ghost = app.world().get::<ComputedLayout>(ghost).unwrap();
    assert!(computed_ghost.child_nodes().is_empty());
    assert!(!computed_ghost.has_layout());

    let computed_fixed = app.world().get::<ComputedLayout>(fixed).unwrap();
    assert!(computed_fixed.has_layout());
    assert!(computed_fixed.is_layout_root());

    let computed_child = app.world().get::<ComputedLayout>(child).unwrap();
    assert!(computed_child.has_layout());
    assert!(computed_child.is_layout_root());

    app.world_mut().entity_mut(ghost).remove::<GhostNode>();
    app.update();

    let computed_former_ghost = app.world().get::<ComputedLayout>(ghost).unwrap();
    assert!(computed_former_ghost
        .child_entities()
        .eq([child].into_iter()));
    assert!(computed_former_ghost.has_layout());

    let computed_fixed = app.world().get::<ComputedLayout>(fixed).unwrap();
    assert!(computed_fixed.has_layout());
    assert!(computed_fixed.is_layout_root());

    let computed_child = app.world().get::<ComputedLayout>(child).unwrap();
    assert!(computed_child.has_layout());
    assert!(!computed_child.is_layout_root());
}

#[test]
fn removing_and_replacing_intermediate_ghost_should_relayout_parent() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let child = world
        .spawn(Node {
            width: px(50.),
            height: px(30.),
            ..default()
        })
        .id();
    let ghost = world
        .spawn((
            Node {
                max_width: px(10.),
                max_height: px(10.),
                flex_grow: 0.,
                ..default()
            },
            GhostNode,
        ))
        .add_child(child)
        .id();
    let root = world.spawn(Node::default()).add_child(ghost).id();

    app.update();

    assert!(app
        .world()
        .get::<ComputedNode>(root)
        .unwrap()
        .size()
        .abs_diff_eq(Vec2::new(50., 30.), 1e-5));

    app.world_mut().entity_mut(ghost).remove::<GhostNode>();
    app.update();

    assert!(app
        .world()
        .get::<ComputedNode>(root)
        .unwrap()
        .size()
        .abs_diff_eq(Vec2::new(10., 10.), 1e-5));

    app.world_mut().entity_mut(ghost).insert(GhostNode);
    app.update();
    assert!(app
        .world()
        .get::<ComputedNode>(root)
        .unwrap()
        .size()
        .abs_diff_eq(Vec2::new(50., 30.), 1e-5));
}

#[test]
fn computed_nodes_of_leaf_nodes_are_updated() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let child = world
        .spawn(Node {
            width: px(50.),
            height: px(30.),
            ..default()
        })
        .id();
    world.spawn(Node::default()).add_child(child);

    app.update();

    assert!(app
        .world()
        .entity(child)
        .get::<ComputedNode>()
        .unwrap()
        .size()
        .abs_diff_eq(Vec2::new(50., 30.), 1e-5));
}

#[test]
fn computed_nodes_of_ghost_parented_leaf_nodes_are_updated() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let child = world
        .spawn(Node {
            width: px(50.),
            height: px(30.),
            ..default()
        })
        .id();
    let ghost = world.spawn(GhostNode).add_child(child).id();
    world.spawn(Node::default()).add_child(ghost);

    app.update();

    assert!(app
        .world()
        .entity(child)
        .get::<ComputedNode>()
        .unwrap()
        .size()
        .abs_diff_eq(Vec2::new(50., 30.), 1e-5));
}

#[test]
fn reflowed_siblings_are_updated() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();

    let child_node = Node {
        width: px(10),
        height: px(10),
        ..default()
    };

    let a = world.spawn(child_node.clone()).id();
    let b = world.spawn(child_node.clone()).id();

    // `RowReverse` aligns the children to the root's right edge,
    // so the root's geometry won't change when we update child a.
    world
        .spawn(Node {
            width: px(100),
            height: px(20),
            flex_direction: FlexDirection::RowReverse,
            ..default()
        })
        .add_children(&[a, b]);

    app.update();
    let world = app.world_mut();

    let a_translation = world.get::<UiGlobalTransform>(a).unwrap().translation;
    let b_translation = world.get::<UiGlobalTransform>(b).unwrap().translation;

    world.get_mut::<Node>(a).unwrap().width = px(20);

    app.update();
    let world = app.world_mut();

    assert_ne!(
        a_translation,
        world.get::<UiGlobalTransform>(a).unwrap().translation
    );
    assert_ne!(
        b_translation,
        world.get::<UiGlobalTransform>(b).unwrap().translation
    );
}

#[test]
fn clipping_updates_when_override_clip_is_inserted_or_removed() {
    let mut app = setup_ui_test_app();

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
    let world = app.world_mut();

    assert!(world.get::<CalculatedClip>(grandchild).is_some());

    world.entity_mut(child).insert(OverrideClip);

    app.update();
    let world = app.world_mut();

    assert!(world.get::<CalculatedClip>(grandchild).is_none());

    world.entity_mut(child).remove::<OverrideClip>();

    app.update();
    let world = app.world_mut();

    assert!(world.get::<CalculatedClip>(grandchild).is_some());
}

#[test]
fn changing_ghost_nodes_ui_transform_translates_child() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();

    let child = world.spawn(Node::default()).id();
    let ghost = world.spawn(GhostNode).add_child(child).id();
    let root = world.spawn(Node::default()).add_child(ghost).id();

    app.update();
    let world = app.world_mut();

    assert_eq!(
        world.get::<UiGlobalTransform>(child).unwrap().translation,
        Vec2::ZERO
    );
    assert_eq!(
        world.get::<UiGlobalTransform>(ghost).unwrap().translation,
        Vec2::ZERO
    );
    assert_eq!(
        world.get::<UiGlobalTransform>(root).unwrap().translation,
        Vec2::ZERO
    );

    let translation = Vec2::new(5., 10.);

    world.get_mut::<UiTransform>(ghost).unwrap().translation =
        Val2::px(translation.x, translation.y);

    app.update();
    let world = app.world_mut();

    assert_eq!(
        world.get::<UiGlobalTransform>(child).unwrap().translation,
        translation
    );
    assert_eq!(
        world.get::<UiGlobalTransform>(ghost).unwrap().translation,
        translation
    );
    assert_eq!(
        world.get::<UiGlobalTransform>(root).unwrap().translation,
        Vec2::ZERO
    );
}

#[test]
fn ghost_nodes_global_transform_persists_update() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();

    let translation = Vec2::new(5., 10.);

    let child = world.spawn(Node::default()).id();
    let ghost = world
        .spawn((
            GhostNode,
            UiTransform::from_translation(Val2::px(translation.x, translation.y)),
        ))
        .add_child(child)
        .id();
    let root = world.spawn(Node::default()).add_child(ghost).id();

    app.update();
    let world = app.world_mut();

    assert_eq!(
        world.get::<UiGlobalTransform>(child).unwrap().translation,
        translation
    );
    assert_eq!(
        world.get::<UiGlobalTransform>(ghost).unwrap().translation,
        translation
    );
    assert_eq!(
        world.get::<UiGlobalTransform>(root).unwrap().translation,
        Vec2::ZERO
    );

    // Spawn another node to trigger a tree update
    world.spawn(Node::default());

    app.update();
    let world = app.world_mut();

    assert_eq!(
        world.get::<UiGlobalTransform>(child).unwrap().translation,
        translation
    );
    assert_eq!(
        world.get::<UiGlobalTransform>(ghost).unwrap().translation,
        translation
    );
    assert_eq!(
        world.get::<UiGlobalTransform>(root).unwrap().translation,
        Vec2::ZERO
    );
}

#[test]
fn nodes_that_are_parented_to_a_non_ui_node_are_cleared() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();

    let node = world
        .spawn(Node {
            width: px(10),
            height: px(10),
            ..default()
        })
        .id();
    let child = world
        .spawn(Node {
            width: px(10),
            height: px(10),
            ..default()
        })
        .id();
    let non_ui_root = world.spawn_empty().add_child(child).id();

    app.update();
    let world = app.world_mut();

    assert_eq!(
        world.get::<ComputedNode>(node).unwrap().size(),
        Vec2::splat(10.)
    );
    assert_eq!(world.get::<ComputedNode>(child).unwrap().size(), Vec2::ZERO);

    world.entity_mut(node).insert(ChildOf(non_ui_root));

    app.update();
    let world = app.world_mut();

    assert_eq!(world.get::<ComputedNode>(node).unwrap().size(), Vec2::ZERO);
    assert_eq!(world.get::<ComputedNode>(child).unwrap().size(), Vec2::ZERO);

    world.entity_mut(non_ui_root).detach_all_children();

    app.update();
    let world = app.world_mut();

    assert_eq!(
        world.get::<ComputedNode>(node).unwrap().size(),
        Vec2::splat(10.)
    );
    assert_eq!(
        world.get::<ComputedNode>(child).unwrap().size(),
        Vec2::splat(10.)
    );
}

#[test]
fn fixed_nodes_that_are_parented_to_a_non_ui_node_are_cleared() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();

    let node = world
        .spawn((
            FixedNode,
            Node {
                width: px(10),
                height: px(10),
                ..default()
            },
        ))
        .id();
    let child = world
        .spawn((
            FixedNode,
            Node {
                width: px(10),
                height: px(10),
                ..default()
            },
        ))
        .id();
    let non_ui_root = world.spawn_empty().add_child(child).id();

    app.update();
    let world = app.world_mut();

    assert_eq!(
        world.get::<ComputedNode>(node).unwrap().size(),
        Vec2::splat(10.)
    );
    assert_eq!(world.get::<ComputedNode>(child).unwrap().size(), Vec2::ZERO);

    world.entity_mut(node).insert(ChildOf(non_ui_root));

    app.update();
    let world = app.world_mut();

    assert_eq!(world.get::<ComputedNode>(node).unwrap().size(), Vec2::ZERO);
    assert_eq!(world.get::<ComputedNode>(child).unwrap().size(), Vec2::ZERO);

    world.entity_mut(non_ui_root).detach_all_children();

    app.update();
    let world = app.world_mut();

    assert_eq!(
        world.get::<ComputedNode>(node).unwrap().size(),
        Vec2::splat(10.)
    );
    assert_eq!(
        world.get::<ComputedNode>(child).unwrap().size(),
        Vec2::splat(10.)
    );
}

#[test]
fn nodes_that_become_ghosts_and_are_detached_at_the_same_time_are_cleared() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();

    let child = world
        .spawn(Node {
            width: px(10),
            height: px(10),
            ..default()
        })
        .id();
    let root = world.spawn(Node::default()).add_child(child).id();

    app.update();
    let world = app.world_mut();

    world.entity_mut(root).detach_all_children();
    world.entity_mut(child).insert(GhostNode);

    app.update();
    let world = app.world_mut();

    assert_eq!(
        world.get::<ComputedNode>(child).unwrap().size(),
        Vec2::splat(0.)
    );
}

#[test]
fn nodes_that_become_ghosts_and_are_parented_to_a_non_ui_node_at_the_same_time_are_cleared() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();

    let node = world
        .spawn(Node {
            width: px(10),
            height: px(10),
            ..default()
        })
        .id();
    let empty_root = world.spawn_empty().id();

    app.update();
    let world = app.world_mut();

    world
        .entity_mut(node)
        .insert((ChildOf(empty_root), GhostNode));

    app.update();
    let world = app.world_mut();

    assert_eq!(
        world.get::<ComputedNode>(node).unwrap().size(),
        Vec2::splat(0.)
    );
}

#[test]
fn root_ghostnode_transform_persists_after_updates() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();
    let ghost_node = world
        .spawn((GhostNode, UiTransform::from_translation(Val2::px(5, 10))))
        .id();
    let child_node = world.spawn((Node::default(), ChildOf(ghost_node))).id();

    app.update();

    let world = app.world_mut();
    assert_eq!(
        world
            .get::<UiGlobalTransform>(ghost_node)
            .unwrap()
            .translation,
        Vec2::new(5., 10.)
    );
    assert_eq!(
        world
            .get::<UiGlobalTransform>(child_node)
            .unwrap()
            .translation,
        Vec2::new(5., 10.)
    );

    world.spawn(Node::default());

    app.update();

    let world = app.world_mut();
    assert_eq!(
        world
            .get::<UiGlobalTransform>(ghost_node)
            .unwrap()
            .translation,
        Vec2::new(5., 10.)
    );
    assert_eq!(
        world
            .get::<UiGlobalTransform>(child_node)
            .unwrap()
            .translation,
        Vec2::new(5., 10.)
    );
}

#[test]
fn scrolling_with_borders_should_clamp_to_padding_box() {
    let mut app = setup_ui_test_app();

    let parent = app
        .world_mut()
        .spawn((
            Node {
                width: px(100.),
                height: px(100.),
                border: px(10.).all(),
                overflow: Overflow::scroll_x(),
                ..default()
            },
            ScrollPosition(Vec2::new(1000., 0.)),
            children![Node {
                min_width: px(200.),
                height: px(100.),
                ..default()
            },],
        ))
        .id();

    app.update();

    // The 10px borders leave a visible space of 80px, so the 200px child can scroll by max 200px - 80px = 120px.
    assert_eq!(
        app.world()
            .get::<ComputedNode>(parent)
            .unwrap()
            .scroll_position,
        Vec2::new(120., 0.)
    );
}

#[test]
fn geometry_updates_skip_clean_subtrees() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let grandchild = world
        .spawn(Node {
            width: px(10.),
            height: px(10.),
            ..default()
        })
        .id();
    let child = world
        .spawn(Node {
            width: px(50.),
            height: px(50.),
            ..default()
        })
        .add_child(grandchild)
        .id();
    world
        .spawn(Node {
            width: px(100.),
            height: px(100.),
            ..default()
        })
        .add_child(child);

    app.update();

    let wrong_translation = Vec2::new(-999., -999.);

    // Update `UiGlobalTransform`'s translation to an obviously wrong value, so we can compare it later to check `grandchild` was updated.
    *app.world_mut()
        .get_mut::<UiGlobalTransform>(grandchild)
        .unwrap() = bevy_math::Affine2::from_translation(wrong_translation).into();

    app.world_mut().entity_mut(child).insert(Node {
        width: px(60.),
        height: px(50.),
        ..default()
    });
    app.update();
    assert_ne!(
        app.world()
            .get::<UiGlobalTransform>(grandchild)
            .unwrap()
            .translation,
        wrong_translation,
        "failed to update a dirty subtree"
    );

    // Nothing is changed this frame, so `grandchild` should not be updated.
    *app.world_mut()
        .get_mut::<UiGlobalTransform>(grandchild)
        .unwrap() = bevy_math::Affine2::from_translation(wrong_translation).into();

    app.update();

    assert_eq!(
        app.world()
            .get::<UiGlobalTransform>(grandchild)
            .unwrap()
            .translation,
        wrong_translation,
        "updated a clean subtree"
    );
}

#[test]
fn percentage_sizes_are_based_on_closest_non_ghost_ancestor() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let descendant2 = world
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: percent(100.),
            height: percent(100.),
            ..default()
        })
        .id();
    let ghost2 = world.spawn(GhostNode).add_child(descendant2).id();
    let descendant1 = world
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: percent(100.),
            height: percent(100.),
            ..default()
        })
        .id();
    let ghost1 = world
        .spawn(GhostNode)
        .add_children(&[ghost2, descendant1])
        .id();
    let root = world
        .spawn(Node {
            width: px(100.),
            height: px(100.),
            ..default()
        })
        .add_child(ghost1)
        .id();

    app.update();

    let world = app.world_mut();
    assert_eq!(world.get::<ComputedNode>(descendant1).unwrap().size.x, 100.);
    assert_eq!(world.get::<ComputedNode>(descendant1).unwrap().size.y, 100.);
    assert_eq!(world.get::<ComputedNode>(descendant2).unwrap().size.x, 100.);
    assert_eq!(world.get::<ComputedNode>(descendant2).unwrap().size.y, 100.);

    world.get_mut::<Node>(root).unwrap().width = px(200.);
    world.get_mut::<Node>(root).unwrap().height = px(300.);

    app.update();

    let world = app.world();

    assert_eq!(world.get::<ComputedNode>(descendant1).unwrap().size.x, 200.);
    assert_eq!(world.get::<ComputedNode>(descendant1).unwrap().size.y, 300.);
    assert_eq!(world.get::<ComputedNode>(descendant2).unwrap().size.x, 200.);
    assert_eq!(world.get::<ComputedNode>(descendant2).unwrap().size.y, 300.);
}

#[test]
fn percentage_sizes_are_updated_after_intermediate_ghost_insertion_and_removal() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let descendant = world
        .spawn(Node {
            width: percent(100.),
            height: percent(100.),
            ..default()
        })
        .id();
    let mid = world
        .spawn((Node {
            width: px(50.),
            height: px(50.),
            ..default()
        },))
        .add_child(descendant)
        .id();
    world
        .spawn(Node {
            width: px(100.),
            height: px(100.),
            ..default()
        })
        .add_child(mid);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.get::<ComputedNode>(descendant).unwrap().size.x, 50.);
    assert_eq!(world.get::<ComputedNode>(descendant).unwrap().size.y, 50.);
    world.entity_mut(mid).insert(GhostNode);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.get::<ComputedNode>(descendant).unwrap().size.x, 100.);
    assert_eq!(world.get::<ComputedNode>(descendant).unwrap().size.y, 100.);
    world.entity_mut(mid).remove::<GhostNode>();

    app.update();

    let world = app.world_mut();
    assert_eq!(world.get::<ComputedNode>(descendant).unwrap().size.x, 50.);
    assert_eq!(world.get::<ComputedNode>(descendant).unwrap().size.y, 50.);
}

#[test]
fn adding_fixednode_clears_inherited_clipping() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let descendant = world.spawn(Node::default()).id();
    let mid = world.spawn(Node::default()).add_child(descendant).id();
    world
        .spawn(Node {
            overflow: Overflow::clip(),
            ..default()
        })
        .add_child(mid);

    app.update();

    let world = app.world_mut();
    assert!(world.get::<CalculatedClip>(descendant).is_some());
    assert!(world.get::<CalculatedClip>(mid).is_some());
    world.entity_mut(mid).insert(FixedNode);

    app.update();

    let world = app.world_mut();
    assert!(world.get::<CalculatedClip>(descendant).is_none());
    assert!(world.get::<CalculatedClip>(mid).is_none());
    world.entity_mut(mid).remove::<FixedNode>();

    app.update();

    let world = app.world();
    assert!(world.get::<CalculatedClip>(descendant).is_some());
    assert!(world.get::<CalculatedClip>(descendant).is_some());
}

#[test]
fn adding_fixednode_updates_sibling() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let child1 = world
        .spawn(Node {
            width: percent(100.),
            height: percent(100.),
            ..default()
        })
        .id();
    let child2 = world
        .spawn(Node {
            width: percent(100.),
            height: percent(100.),
            ..default()
        })
        .id();

    world
        .spawn(Node {
            width: px(100.),
            height: px(100.),
            ..default()
        })
        .add_children(&[child1, child2]);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.get::<ComputedNode>(child1).unwrap().size.x, 50.);
    assert_eq!(world.get::<ComputedNode>(child2).unwrap().size.x, 50.);
    world.entity_mut(child1).insert(FixedNode);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.get::<ComputedNode>(child1).unwrap().size.x, 1000.);
    assert_eq!(world.get::<ComputedNode>(child2).unwrap().size.x, 100.);
    world.entity_mut(child1).remove::<FixedNode>();

    app.update();

    let world = app.world_mut();
    assert_eq!(world.get::<ComputedNode>(child1).unwrap().size.x, 50.);
    assert_eq!(world.get::<ComputedNode>(child2).unwrap().size.x, 50.);
    world.entity_mut(child2).insert(FixedNode);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.get::<ComputedNode>(child1).unwrap().size.x, 100.);
    assert_eq!(world.get::<ComputedNode>(child2).unwrap().size.x, 1000.);
    world.entity_mut(child1).insert(FixedNode);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.get::<ComputedNode>(child1).unwrap().size.x, 1000.);
    assert_eq!(world.get::<ComputedNode>(child2).unwrap().size.x, 1000.);
}

#[test]
fn adding_fixednode_updates_sibling_clipping() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let descendant = world.spawn(Node::default()).id();
    let child1 = world
        .spawn(Node {
            width: percent(100.),
            height: percent(100.),
            ..default()
        })
        .id();
    let child2 = world
        .spawn(Node {
            width: percent(100.),
            height: percent(100.),
            overflow: Overflow::clip(),
            ..default()
        })
        .add_child(descendant)
        .id();
    world
        .spawn(Node {
            width: px(100.),
            height: px(100.),
            ..default()
        })
        .add_children(&[child1, child2]);

    app.update();

    let world = app.world_mut();
    assert_eq!(world.get::<ComputedNode>(child2).unwrap().size.x, 50.);
    assert_eq!(
        world
            .get::<CalculatedClip>(descendant)
            .unwrap()
            .rects()
            .unwrap()[0]
            .rect
            .width(),
        50.
    );
    world.entity_mut(child1).insert(FixedNode);

    app.update();

    let world = app.world();
    assert_eq!(world.get::<ComputedNode>(child2).unwrap().size.x, 100.);
    assert_eq!(
        world
            .get::<CalculatedClip>(descendant)
            .unwrap()
            .rects()
            .unwrap()[0]
            .rect
            .width(),
        100.
    );
}

#[test]
fn node_parented_to_empty_root_should_be_cleared() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let node = world
        .spawn(Node {
            width: px(10),
            height: px(10),
            ..default()
        })
        .id();
    let empty_root = world.spawn_empty().id();

    app.update();

    let world = app.world_mut();
    assert_eq!(
        world.get::<ComputedNode>(node).unwrap().size(),
        Vec2::splat(10.)
    );
    world.get_mut::<Node>(node).unwrap().width = px(20);

    app.update();

    let world = app.world_mut();
    assert_eq!(
        world.get::<ComputedNode>(node).unwrap().size(),
        Vec2::new(20., 10.)
    );
    world.entity_mut(node).insert(ChildOf(empty_root));

    app.update();

    assert_eq!(
        app.world().get::<ComputedNode>(node).unwrap().size(),
        Vec2::ZERO
    );
}

#[test]
fn computed_node_is_unchanged_after_ui_transform_updated() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let node = world
        .spawn(Node {
            width: px(100),
            height: px(100),
            ..default()
        })
        .id();

    let entity_ref = world.entity(node);

    let c0 = entity_ref.get_ref::<ComputedNode>().unwrap().last_changed();
    let t0 = entity_ref
        .get_ref::<UiGlobalTransform>()
        .unwrap()
        .last_changed();

    app.update();

    let world = app.world_mut();
    let entity_ref = world.entity(node);
    let c1 = entity_ref.get_ref::<ComputedNode>().unwrap().last_changed();
    let t1 = entity_ref
        .get_ref::<UiGlobalTransform>()
        .unwrap()
        .last_changed();

    assert_ne!(c0, c1);
    assert_ne!(t0, t1);

    app.update();

    let world = app.world_mut();
    let mut entity_mut = world.entity_mut(node);
    let c2 = entity_mut.get_ref::<ComputedNode>().unwrap().last_changed();
    let t2 = entity_mut
        .get_ref::<UiGlobalTransform>()
        .unwrap()
        .last_changed();

    assert_eq!(c1, c2);
    assert_eq!(t1, t2);

    entity_mut.insert(UiTransform::from_translation(Val2::px(10, 5)));

    app.update();

    let world = app.world_mut();
    let entity_mut = world.entity_mut(node);
    let c3 = entity_mut.get_ref::<ComputedNode>().unwrap().last_changed();
    let t3 = entity_mut
        .get_ref::<UiGlobalTransform>()
        .unwrap()
        .last_changed();

    assert_eq!(c2, c3);
    assert_ne!(t2, t3);
}

#[test]
fn child_layout_change_does_not_mark_parent_self_dirty() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let child = world.spawn(Node::default()).id();
    let parent = world.spawn(Node::default()).add_child(child).id();

    app.update();

    app.world_mut().get_mut::<Node>(child).unwrap().width = px(10);

    app.update();

    let computed_layout = app.world().get::<ComputedLayout>(parent).unwrap();
    assert!(computed_layout.subtree_dirty());
    assert!(!computed_layout.self_dirty());
}

#[test]
fn child_layout_change_does_not_update_clean_sibling() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let child = world
        .spawn(Node {
            position_type: PositionType::Absolute,
            ..default()
        })
        .id();
    let sibling = world.spawn(Node::default()).id();
    world
        .spawn(Node {
            width: px(100),
            height: px(100),
            ..default()
        })
        .add_children(&[child, sibling]);

    app.update();

    let world = app.world_mut();
    let wrong_translation = Vec2::splat(-999.);
    *world.get_mut::<UiGlobalTransform>(sibling).unwrap() =
        bevy_math::Affine2::from_translation(wrong_translation).into();
    world.get_mut::<Node>(child).unwrap().width = px(10);

    app.update();

    let world = app.world();
    assert_eq!(world.get::<ComputedNode>(child).unwrap().size().x, 10.);
    assert_eq!(
        world.get::<UiGlobalTransform>(sibling).unwrap().translation,
        wrong_translation
    );
}

#[test]
fn clipping_updates_skip_clean_subtrees() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let child = world.spawn(Node::default()).id();
    let sibling = world.spawn(Node::default()).id();
    world
        .spawn(Node {
            overflow: Overflow::clip(),
            ..default()
        })
        .add_children(&[child, sibling]);

    app.update();

    // Overwrite `sibling`'s clipping so we can check whether it gets recomputed.
    *app.world_mut().get_mut::<CalculatedClip>(sibling).unwrap() = CalculatedClip::FullyClipped;

    // `child` has no children, so setting a `ScrollPosition` should do nothing.
    app.world_mut()
        .entity_mut(child)
        .insert(ScrollPosition(Vec2::splat(50.)));
    app.update();

    assert!(app
        .world()
        .get::<CalculatedClip>(sibling)
        .unwrap()
        .is_fully_clipped());
}

#[test]
fn computed_node_for_unreachable_node_should_be_unchanged() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let unreachable = world.spawn(Node::default()).id();
    let non_ui_node = world.spawn_empty().add_child(unreachable).id();
    world.spawn(Node::default()).add_child(non_ui_node);

    app.update();

    let t = app
        .world()
        .entity(unreachable)
        .get_ref::<ComputedNode>()
        .unwrap()
        .last_changed();
    app.world_mut().spawn(Node::default());

    app.update();

    assert_eq!(
        t,
        app.world()
            .entity(unreachable)
            .get_ref::<ComputedNode>()
            .unwrap()
            .last_changed()
    );
}

#[test]
fn computed_layout_for_unreachable_node_should_be_clean() {
    let mut app = setup_ui_test_app();
    let world = app.world_mut();
    let unreachable = world.spawn(Node::default()).id();
    let non_ui_node = world.spawn_empty().add_child(unreachable).id();
    world.spawn(Node::default()).add_child(non_ui_node);

    app.update();

    app.world_mut().spawn(Node::default());

    app.update();

    let c = app
        .world()
        .entity(unreachable)
        .get_ref::<ComputedLayout>()
        .unwrap();
    assert!(!c.self_dirty());
    assert!(!c.subtree_dirty());
    assert!(!c.layout_dirty());
}

#[test]
fn changing_ghost_nodes_ui_transform_updates_descendant_clipping() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();
    let descendant = world.spawn(Node::default()).id();
    let clipper = world
        .spawn(Node {
            width: px(10),
            height: px(10),
            overflow: Overflow::clip(),
            ..default()
        })
        .add_child(descendant)
        .id();
    let ghost = world.spawn(GhostNode).add_child(clipper).id();
    world
        .spawn(Node {
            width: px(100),
            height: px(100),
            ..default()
        })
        .add_child(ghost);

    app.update();

    let clip = app
        .world()
        .get::<CalculatedClip>(descendant)
        .unwrap()
        .clone();
    app.world_mut()
        .get_mut::<UiTransform>(ghost)
        .unwrap()
        .translation = Val2::px(20., 0.);

    app.update();

    assert!(clip != *app.world().get::<CalculatedClip>(descendant).unwrap());
}

#[test]
fn rounding_is_updated_on_hierarchy_changes_even_if_unrounded_layout_not_changed() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();
    let a = world
        .spawn(Node {
            width: px(20.),
            height: px(20.),
            left: px(0.1),
            ..default()
        })
        .id();
    let b = world
        .spawn(Node {
            width: px(20.),
            height: px(20.),
            left: px(0.6),
            ..default()
        })
        .id();
    let c = world
        .spawn((
            Node {
                width: px(10.6),
                height: px(10.6),
                ..default()
            },
            ChildOf(a),
        ))
        .id();
    let d = world
        .spawn((
            Node {
                width: px(10.6),
                height: px(10.6),
                ..default()
            },
            ChildOf(b),
        ))
        .id();

    app.update();

    let world = app.world_mut();

    // (0.1 + 10.6).round() - 0.1.round() = 11 - 0 = 11
    assert_eq!(world.get::<ComputedNode>(c).unwrap().size().x, 11.);
    // (0.6 + 10.6).round() - 0.6.round() = 11 - 1 = 10
    assert_eq!(world.get::<ComputedNode>(d).unwrap().size().x, 10.);
    world.entity_mut(c).insert(ChildOf(b));
    world.entity_mut(d).insert(ChildOf(a));

    app.update();

    let world = app.world_mut();

    assert_eq!(world.get::<ComputedNode>(c).unwrap().size().x, 10.);
    assert_eq!(world.get::<ComputedNode>(d).unwrap().size().x, 11.);
}

#[test]
fn display_none_on_a_ghost_node_is_ignored() {
    let mut app = setup_ui_test_app();

    let world = app.world_mut();
    let root = world.spawn(Node::default()).id();
    let child = world
        .spawn((
            Node {
                width: px(10.),
                height: px(10.),
                ..default()
            },
            ChildOf(root),
        ))
        .id();

    app.update();

    let world = app.world_mut();
    assert_eq!(
        world.get::<ComputedNode>(child).unwrap().size,
        Vec2::splat(10.)
    );
    world.get_mut::<Node>(root).unwrap().display = Display::None;

    app.update();

    let world = app.world_mut();
    assert_eq!(world.get::<ComputedNode>(child).unwrap().size, Vec2::ZERO);
    world.entity_mut(root).insert(GhostNode);

    app.update();

    let world = app.world_mut();
    assert_eq!(
        world.get::<ComputedNode>(child).unwrap().size,
        Vec2::splat(10.)
    );
    assert_eq!(
        world.get::<ComputedNode>(root).unwrap().size,
        Vec2::splat(0.)
    );
}
