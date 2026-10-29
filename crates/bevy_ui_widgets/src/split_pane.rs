use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityHashSet},
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy_math::Vec2;
use bevy_picking::{
    events::{PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart},
    hover::PointerCaptureMap,
    pointer::PointerButton,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{
    px, ComputedNode, Display, FlexDirection, InteractionDisabled, Node, UiScale, UiSystems,
};

use crate::{ControlOrientation, ValueChange};

/// A container that divides its space between [`Pane`] children along one axis.
///
/// Children are panes separated by [`SplitPaneHandle`]s. Dragging a handle emits
/// [`ValueChange<Vec<f32>>`] from this entity, carrying the proposed [`Pane::size`] of every pane
/// in child order. Only the two panes next to the handle change. Sizes are not updated unless
/// [`split_pane_self_update`] is attached as an observer.
///
/// [`InteractionDisabled`] on this entity or on a handle disables dragging. The container's
/// [`Node::flex_direction`] and each pane's flex properties are set from these components.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct SplitPane {
    /// The axis along which panes are laid out.
    pub orientation: ControlOrientation,
}

impl Default for SplitPane {
    fn default() -> Self {
        Self {
            orientation: ControlOrientation::Horizontal,
        }
    }
}

/// A resizable child of a [`SplitPane`].
///
/// Each visible pane receives a share of the container proportional to its `size`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct Pane {
    /// The flex weight of this pane along the split axis.
    pub size: f32,
    /// The minimum size along the split axis, in logical pixels.
    pub min_size: f32,
}

impl Default for Pane {
    fn default() -> Self {
        Self {
            size: 1.0,
            min_size: 0.0,
        }
    }
}

/// A draggable divider between two [`Pane`]s of a [`SplitPane`].
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[require(
    AccessibilityNode(accesskit::Node::new(Role::Splitter)),
    SplitPaneDragState
)]
#[reflect(Component, Default, Clone)]
pub struct SplitPaneHandle;

/// The drag state of a [`SplitPaneHandle`].
#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct SplitPaneDragState {
    /// Whether the handle is currently being dragged.
    pub dragging: bool,
    start_sizes: Vec<f32>,
    before: usize,
    after: usize,
    before_px: f32,
    pair_px: f32,
    min_delta: f32,
    max_delta: f32,
}

impl SplitPaneDragState {
    fn begin(
        handle: Entity,
        children: &Children,
        orientation: ControlOrientation,
        q_pane: &Query<(&Pane, Option<&Node>, Option<&ComputedNode>)>,
    ) -> Option<Self> {
        let mut start_sizes = Vec::new();
        let mut before = None;
        let mut after = None;
        let mut past_handle = false;
        for &child in children {
            if child == handle {
                past_handle = true;
                continue;
            }
            let Ok((pane, node, computed)) = q_pane.get(child) else {
                continue;
            };
            let index = start_sizes.len();
            start_sizes.push(pane.size);
            if node.is_some_and(|node| node.display == Display::None) {
                continue;
            }
            let size = computed
                .map(|c| c.size() * c.inverse_scale_factor())
                .unwrap_or(Vec2::ZERO);
            let extent = match orientation {
                ControlOrientation::Horizontal => size.x,
                ControlOrientation::Vertical => size.y,
            };
            if !past_handle {
                before = Some((index, extent, pane.min_size));
            } else if after.is_none() {
                after = Some((index, extent, pane.min_size));
            }
        }
        let (before, before_px, before_min) = before?;
        let (after, after_px, after_min) = after?;
        Some(Self {
            dragging: true,
            start_sizes,
            before,
            after,
            before_px,
            pair_px: before_px + after_px,
            min_delta: before_min - before_px,
            max_delta: after_px - after_min,
        })
    }

    fn proposed_sizes(&self, distance: f32) -> Vec<f32> {
        let mut sizes = self.start_sizes.clone();
        if self.pair_px > 0.0 {
            let delta = distance.max(self.min_delta).min(self.max_delta);
            let pair_size = sizes[self.before] + sizes[self.after];
            sizes[self.before] = pair_size * (self.before_px + delta) / self.pair_px;
            sizes[self.after] = pair_size - sizes[self.before];
        }
        sizes
    }
}

fn drag_distance(distance: Vec2, orientation: ControlOrientation, ui_scale: &UiScale) -> f32 {
    let distance = distance / ui_scale.0;
    match orientation {
        ControlOrientation::Horizontal => distance.x,
        ControlOrientation::Vertical => distance.y,
    }
}

fn split_pane_on_drag_start(
    mut drag_start: On<PointerDragStart>,
    mut q_handle: Query<
        (&ChildOf, &mut SplitPaneDragState, Has<InteractionDisabled>),
        With<SplitPaneHandle>,
    >,
    q_split: Query<(&SplitPane, &Children, Has<InteractionDisabled>)>,
    q_pane: Query<(&Pane, Option<&Node>, Option<&ComputedNode>)>,
    mut capture_map: ResMut<PointerCaptureMap>,
) {
    if drag_start.button != PointerButton::Primary {
        return;
    }
    let Ok((ChildOf(parent), mut state, handle_disabled)) = q_handle.get_mut(drag_start.entity)
    else {
        return;
    };
    let Ok((split, children, disabled)) = q_split.get(*parent) else {
        return;
    };
    if disabled || handle_disabled {
        return;
    }
    let Some(origin) =
        SplitPaneDragState::begin(drag_start.entity, children, split.orientation, &q_pane)
    else {
        return;
    };
    drag_start.propagate(false);
    *state = origin;
    capture_map.capture(
        drag_start.pointer.id,
        drag_start.entity,
        drag_start.hit.clone(),
    );
}

fn split_pane_on_drag(
    mut drag: On<PointerDrag>,
    q_handle: Query<(&ChildOf, &SplitPaneDragState), With<SplitPaneHandle>>,
    q_split: Query<(&SplitPane, Has<InteractionDisabled>)>,
    ui_scale: Res<UiScale>,
    mut commands: Commands,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let Ok((ChildOf(parent), state)) = q_handle.get(drag.entity) else {
        return;
    };
    let Ok((split, disabled)) = q_split.get(*parent) else {
        return;
    };
    if !state.dragging || disabled {
        return;
    }
    drag.propagate(false);
    let distance = drag_distance(drag.distance, split.orientation, &ui_scale);
    commands.trigger(ValueChange {
        source: *parent,
        value: state.proposed_sizes(distance),
        is_final: false,
    });
}

fn split_pane_on_drag_end(
    mut drag_end: On<PointerDragEnd>,
    mut q_handle: Query<(&ChildOf, &mut SplitPaneDragState), With<SplitPaneHandle>>,
    q_split: Query<(&SplitPane, Has<InteractionDisabled>)>,
    ui_scale: Res<UiScale>,
    mut capture_map: ResMut<PointerCaptureMap>,
    mut commands: Commands,
) {
    if drag_end.button != PointerButton::Primary {
        return;
    }
    let Ok((ChildOf(parent), mut state)) = q_handle.get_mut(drag_end.entity) else {
        return;
    };
    if !state.dragging {
        return;
    }
    drag_end.propagate(false);
    state.dragging = false;
    capture_map.release(drag_end.pointer.id);
    let Ok((split, disabled)) = q_split.get(*parent) else {
        return;
    };
    let value = if disabled {
        state.start_sizes.clone()
    } else {
        state.proposed_sizes(drag_distance(
            drag_end.distance,
            split.orientation,
            &ui_scale,
        ))
    };
    commands.trigger(ValueChange {
        source: *parent,
        value,
        is_final: true,
    });
}

fn split_pane_on_cancel(
    mut cancel: On<PointerCancel>,
    mut q_handle: Query<(&ChildOf, &mut SplitPaneDragState), With<SplitPaneHandle>>,
    mut capture_map: ResMut<PointerCaptureMap>,
    mut commands: Commands,
) {
    let Ok((ChildOf(parent), mut state)) = q_handle.get_mut(cancel.entity) else {
        return;
    };
    if !state.dragging {
        return;
    }
    cancel.propagate(false);
    state.dragging = false;
    capture_map.release(cancel.pointer.id);
    commands.trigger(ValueChange {
        source: *parent,
        value: state.start_sizes.clone(),
        is_final: true,
    });
}

fn update_split_pane_layout(
    q_changed_split: Query<Entity, (With<SplitPane>, Or<(Changed<SplitPane>, Changed<Children>)>)>,
    q_changed_pane: Query<&ChildOf, Changed<Pane>>,
    q_split: Query<(&SplitPane, &Children)>,
    q_pane: Query<&Pane>,
    mut q_node: Query<&mut Node>,
) {
    let mut dirty = EntityHashSet::default();
    dirty.extend(&q_changed_split);
    dirty.extend(q_changed_pane.iter().map(ChildOf::parent));
    for split_entity in dirty {
        let Ok((split, children)) = q_split.get(split_entity) else {
            continue;
        };
        if let Ok(mut node) = q_node.get_mut(split_entity) {
            node.flex_direction = match split.orientation {
                ControlOrientation::Horizontal => FlexDirection::Row,
                ControlOrientation::Vertical => FlexDirection::Column,
            };
        }
        for &child in children {
            let (Ok(pane), Ok(mut node)) = (q_pane.get(child), q_node.get_mut(child)) else {
                continue;
            };
            node.flex_grow = pane.size.max(0.0);
            node.flex_basis = px(0);
            match split.orientation {
                ControlOrientation::Horizontal => node.min_width = px(pane.min_size),
                ControlOrientation::Vertical => node.min_height = px(pane.min_size),
            }
        }
    }
}

/// Observer that applies [`ValueChange<Vec<f32>>`] from a [`SplitPane`] to its [`Pane`] sizes.
pub fn split_pane_self_update(
    change: On<ValueChange<Vec<f32>>>,
    q_split: Query<&Children, With<SplitPane>>,
    mut q_pane: Query<&mut Pane>,
) {
    let Ok(children) = q_split.get(change.source) else {
        return;
    };
    let mut sizes = change.value.iter();
    for &child in children {
        let Ok(mut pane) = q_pane.get_mut(child) else {
            continue;
        };
        let Some(&size) = sizes.next() else {
            break;
        };
        if pane.size != size {
            pane.size = size;
        }
    }
}

/// Plugin that registers the observers and layout system for [`SplitPane`].
pub struct SplitPanePlugin;

impl Plugin for SplitPanePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(split_pane_on_drag_start)
            .add_observer(split_pane_on_drag)
            .add_observer(split_pane_on_drag_end)
            .add_observer(split_pane_on_cancel)
            .add_systems(
                PostUpdate,
                update_split_pane_layout.before(UiSystems::Prepare),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_camera::NormalizedRenderTarget;
    use bevy_ecs::resource::Resource;
    use bevy_picking::{
        backend::HitData,
        events::Pointer,
        pointer::{Location, PointerId},
    };

    #[derive(Resource, Default)]
    struct Changes(Vec<(Vec<f32>, bool)>);

    fn split_app() -> App {
        let mut app = App::new();
        app.add_plugins(SplitPanePlugin)
            .init_resource::<PointerCaptureMap>()
            .init_resource::<UiScale>()
            .init_resource::<Changes>()
            .add_observer(
                |change: On<ValueChange<Vec<f32>>>, mut changes: ResMut<Changes>| {
                    changes.0.push((change.value.clone(), change.is_final));
                },
            );
        app
    }

    fn spawn_split(
        app: &mut App,
        orientation: ControlOrientation,
        panes: &[(f32, f32, f32)],
    ) -> (Entity, Vec<Entity>, Vec<Entity>) {
        let world = app.world_mut();
        let split = world
            .spawn((SplitPane { orientation }, Node::default()))
            .id();
        let mut pane_entities = Vec::new();
        let mut handles = Vec::new();
        for (index, &(size, min_size, extent)) in panes.iter().enumerate() {
            if index > 0 {
                handles.push(world.spawn((SplitPaneHandle, ChildOf(split))).id());
            }
            let computed_size = match orientation {
                ControlOrientation::Horizontal => Vec2::new(extent, 100.0),
                ControlOrientation::Vertical => Vec2::new(100.0, extent),
            };
            pane_entities.push(
                world
                    .spawn((
                        Pane { size, min_size },
                        Node::default(),
                        ComputedNode {
                            size: computed_size,
                            inverse_scale_factor: 1.0,
                            ..Default::default()
                        },
                        ChildOf(split),
                    ))
                    .id(),
            );
        }
        (split, handles, pane_entities)
    }

    fn pointer() -> Pointer {
        Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 0,
                    height: 0,
                },
                position: Vec2::ZERO,
            },
        )
    }

    fn hit() -> HitData {
        HitData {
            camera: Entity::PLACEHOLDER,
            depth: 0.0,
            position: None,
            normal: None,
            extra: None,
        }
    }

    fn drag(app: &mut App, handle: Entity, distance: Vec2) {
        let world = app.world_mut();
        world.trigger(PointerDragStart {
            entity: handle,
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: hit(),
        });
        world.trigger(PointerDrag {
            entity: handle,
            pointer: pointer(),
            button: PointerButton::Primary,
            distance,
            delta: distance,
        });
        world.trigger(PointerDragEnd {
            entity: handle,
            pointer: pointer(),
            button: PointerButton::Primary,
            distance,
        });
        world.flush();
    }

    fn last_change(app: &App) -> Option<(Vec<f32>, bool)> {
        app.world().resource::<Changes>().0.last().cloned()
    }

    fn assert_sizes(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 1e-4, "{actual:?} != {expected:?}");
        }
    }

    fn pane_size(app: &App, pane: Entity) -> f32 {
        app.world().get::<Pane>(pane).unwrap().size
    }

    #[test]
    fn drag_resizes_adjacent_panes() {
        let mut app = split_app();
        let (_, handles, panes) = spawn_split(
            &mut app,
            ControlOrientation::Horizontal,
            &[(1.0, 0.0, 100.0), (1.0, 0.0, 100.0)],
        );
        drag(&mut app, handles[0], Vec2::new(50.0, 20.0));

        let changes = &app.world().resource::<Changes>().0;
        assert_eq!(changes.len(), 2);
        assert!(!changes[0].1);
        let (sizes, is_final) = last_change(&app).unwrap();
        assert!(is_final);
        assert_sizes(&sizes, &[1.5, 0.5]);
        assert_eq!(pane_size(&app, panes[0]), 1.0);
        assert!(app.world().resource::<PointerCaptureMap>().0.is_empty());
    }

    #[test]
    fn drag_clamps_to_min_size() {
        let mut app = split_app();
        let (_, handles, _) = spawn_split(
            &mut app,
            ControlOrientation::Horizontal,
            &[(1.0, 80.0, 100.0), (1.0, 80.0, 100.0)],
        );
        drag(&mut app, handles[0], Vec2::new(90.0, 0.0));
        assert_sizes(&last_change(&app).unwrap().0, &[1.2, 0.8]);

        drag(&mut app, handles[0], Vec2::new(-90.0, 0.0));
        assert_sizes(&last_change(&app).unwrap().0, &[0.8, 1.2]);
    }

    #[test]
    fn vertical_split_uses_y_axis() {
        let mut app = split_app();
        let (_, handles, _) = spawn_split(
            &mut app,
            ControlOrientation::Vertical,
            &[(1.0, 0.0, 100.0), (1.0, 0.0, 100.0)],
        );
        drag(&mut app, handles[0], Vec2::new(50.0, 30.0));
        assert_sizes(&last_change(&app).unwrap().0, &[1.3, 0.7]);
    }

    #[test]
    fn disabled_split_ignores_drag() {
        let mut app = split_app();
        let (split, handles, _) = spawn_split(
            &mut app,
            ControlOrientation::Horizontal,
            &[(1.0, 0.0, 100.0), (1.0, 0.0, 100.0)],
        );
        app.world_mut()
            .entity_mut(split)
            .insert(InteractionDisabled);
        drag(&mut app, handles[0], Vec2::new(50.0, 0.0));
        assert!(app.world().resource::<Changes>().0.is_empty());
        assert!(
            !app.world()
                .get::<SplitPaneDragState>(handles[0])
                .unwrap()
                .dragging
        );
    }

    #[test]
    fn self_update_applies_sizes_and_layout() {
        let mut app = split_app();
        app.add_observer(split_pane_self_update);
        let (split, handles, panes) = spawn_split(
            &mut app,
            ControlOrientation::Horizontal,
            &[(1.0, 20.0, 100.0), (1.0, 0.0, 100.0)],
        );
        drag(&mut app, handles[0], Vec2::new(50.0, 0.0));
        assert_eq!(pane_size(&app, panes[0]), 1.5);
        assert_eq!(pane_size(&app, panes[1]), 0.5);

        app.update();
        let world = app.world();
        assert_eq!(
            world.get::<Node>(split).unwrap().flex_direction,
            FlexDirection::Row
        );
        let node = world.get::<Node>(panes[0]).unwrap();
        assert_eq!(node.flex_grow, 1.5);
        assert_eq!(node.flex_basis, px(0));
        assert_eq!(node.min_width, px(20));
        assert_eq!(world.get::<Node>(panes[1]).unwrap().flex_grow, 0.5);
    }

    #[test]
    fn three_panes_only_change_adjacent_pair() {
        let mut app = split_app();
        let (_, handles, _) = spawn_split(
            &mut app,
            ControlOrientation::Horizontal,
            &[(1.0, 0.0, 50.0), (2.0, 0.0, 100.0), (1.0, 0.0, 50.0)],
        );
        drag(&mut app, handles[1], Vec2::new(-25.0, 0.0));
        assert_sizes(&last_change(&app).unwrap().0, &[1.0, 1.5, 1.5]);
    }

    #[test]
    fn cancel_restores_start_sizes() {
        let mut app = split_app();
        let (_, handles, _) = spawn_split(
            &mut app,
            ControlOrientation::Horizontal,
            &[(1.0, 0.0, 100.0), (3.0, 0.0, 300.0)],
        );
        let world = app.world_mut();
        world.trigger(PointerDragStart {
            entity: handles[0],
            pointer: pointer(),
            button: PointerButton::Primary,
            hit: hit(),
        });
        world.trigger(PointerCancel {
            entity: handles[0],
            pointer: pointer(),
            hit: hit(),
        });
        world.flush();
        assert_eq!(last_change(&app), Some((vec![1.0, 3.0], true)));
    }
}
