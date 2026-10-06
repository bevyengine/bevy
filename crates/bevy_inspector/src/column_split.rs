//! A draggable boundary between the leading and trailing columns of the rows in a container.

use alloc::vec::Vec;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    change_detection::DetectChanges,
    children,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::Add,
    observer::On,
    query::{Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, ParamSet, Query, Res, ResMut},
    world::Ref,
};
use bevy_feathers::{theme::ThemeBackgroundColor, tokens};
use bevy_picking::{
    cursor::EntityCursor,
    events::{PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart},
    hover::{Hovered, PointerCaptureMap},
    pointer::PointerButton,
    Pickable,
};
use bevy_platform::collections::HashMap;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{
    px, ComputedNode, InteractionDisabled, JustifyContent, Node, PositionType, UiGlobalTransform,
    UiScale, UiSystems, Val, ZIndex,
};
use bevy_window::SystemCursorIcon;

const HANDLE_WIDTH: f32 = 5.0;

/// A container whose descendant rows share one boundary between a leading and a trailing column.
///
/// Rows mark their leading column with [`ColumnSplitLeading`]. A [`ColumnSplitHandle`] child is
/// spawned when this component is added, drawn as a line along the boundary over the full height
/// of the container. Dragging it writes `fraction` directly, without emitting a change event.
/// [`InteractionDisabled`] on this entity disables dragging.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[require(Node, ColumnSplitDragState)]
#[reflect(Component, Default, Debug, Clone, PartialEq)]
pub struct ColumnSplit {
    /// The x position of the boundary, as a fraction of this node's content width.
    pub fraction: f32,
    /// The smallest allowed `fraction`.
    pub min: f32,
    /// The largest allowed `fraction`.
    pub max: f32,
}

impl Default for ColumnSplit {
    fn default() -> Self {
        Self {
            fraction: 0.4,
            min: 0.15,
            max: 0.75,
        }
    }
}

impl ColumnSplit {
    /// `fraction`, clamped between `min` and `max`.
    pub fn clamped(&self) -> f32 {
        self.fraction.min(self.max).max(self.min)
    }
}

#[derive(Component, Debug, Default, Clone)]
struct ColumnSplitDragState {
    dragging: bool,
    start: f32,
    width: f32,
}

/// Marker for the leading column of a row, whose width is set so that it ends at the left edge
/// of the nearest ancestor [`ColumnSplit`]'s handle, whatever the row's indent.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[require(Node)]
#[reflect(Component, Default, Debug, Clone)]
pub struct ColumnSplitLeading;

/// The draggable line of a [`ColumnSplit`], spawned as its child.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[require(Node, Hovered)]
#[reflect(Component, Default, Debug, Clone)]
pub struct ColumnSplitHandle;

#[derive(Component, Debug, Default, Clone, Copy)]
struct ColumnSplitHandleLine;

fn spawn_handle(add: On<Add<ColumnSplit>>, mut commands: Commands) {
    commands.spawn((
        ColumnSplitHandle,
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            bottom: px(0),
            width: px(HANDLE_WIDTH),
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        ZIndex(1),
        EntityCursor::System(SystemCursorIcon::ColResize),
        ChildOf(add.entity),
        children![(
            ColumnSplitHandleLine,
            Node {
                width: px(1),
                ..Default::default()
            },
            ThemeBackgroundColor(tokens::SPLIT_PANE_HANDLE),
            Pickable::IGNORE,
        )],
    ));
}

type Geometry<'a> = (
    Ref<'a, ComputedNode>,
    &'a UiGlobalTransform,
    &'a Node,
    Option<&'a ChildOf>,
);

fn px_value(val: Val) -> f32 {
    match val {
        Val::Px(value) => value,
        _ => 0.0,
    }
}

/// The left and right edges of `entity`'s border box, in physical pixels.
///
/// Nodes that have not been laid out yet are assumed to stretch across their parent's content
/// box, so that newly spawned rows line up on their first frame.
fn border_box(entity: Entity, nodes: &Query<Geometry>, scale: f32) -> Option<[f32; 2]> {
    let (computed, transform, node, parent) = nodes.get(entity).ok()?;
    if !computed.is_added() {
        let half = 0.5 * computed.size.x;
        return Some([
            transform.translation.x - half,
            transform.translation.x + half,
        ]);
    }
    let [left, right] = inner_box(parent?.parent(), nodes, scale, true)?;
    let margin = &node.margin;
    Some([
        left + px_value(margin.left) * scale,
        right - px_value(margin.right) * scale,
    ])
}

/// The edges of `entity`'s content box, or of its padding box if `padding` is false.
fn inner_box(
    entity: Entity,
    nodes: &Query<Geometry>,
    scale: f32,
    padding: bool,
) -> Option<[f32; 2]> {
    let [left, right] = border_box(entity, nodes, scale)?;
    let (computed, _, node, _) = nodes.get(entity).ok()?;
    let side =
        |border: Val, pad: Val| (px_value(border) + f32::from(padding) * px_value(pad)) * scale;
    let inset = match padding {
        true => computed.content_inset(),
        false => computed.border,
    };
    let [inset_left, inset_right] = match computed.is_added() {
        true => [
            side(node.border.left, node.padding.left),
            side(node.border.right, node.padding.right),
        ],
        false => [inset.min_inset.x, inset.max_inset.x],
    };
    Some([left + inset_left, right - inset_right])
}

/// The number of physical pixels per logical pixel, from the nearest laid out node.
fn scale_factor(mut entity: Entity, nodes: &Query<Geometry>) -> Option<f32> {
    loop {
        let (computed, _, _, parent) = nodes.get(entity).ok()?;
        if !computed.is_added() {
            return Some(computed.inverse_scale_factor.recip());
        }
        entity = parent?.parent();
    }
}

fn update_column_splits(
    q_splits: Query<(Entity, &ColumnSplit)>,
    q_handles: Query<(Entity, &ChildOf), With<ColumnSplitHandle>>,
    q_leading: Query<Entity, With<ColumnSplitLeading>>,
    q_parents: Query<&ChildOf>,
    mut nodes: ParamSet<(
        Query<Geometry>,
        Query<&mut Node, Or<(With<ColumnSplitLeading>, With<ColumnSplitHandle>)>>,
    )>,
) {
    let geometry = nodes.p0();
    let mut boundaries = HashMap::new();
    for (entity, split) in &q_splits {
        if let Some(scale) = scale_factor(entity, &geometry)
            && let Some([left, right]) = inner_box(entity, &geometry, scale, true)
            && let Some([padding_left, _]) = inner_box(entity, &geometry, scale, false)
        {
            let boundary = left + split.clamped() * (right - left);
            boundaries.insert(entity, (boundary, padding_left, scale));
        }
    }

    let mut offsets = Vec::new();
    for (handle, child_of) in &q_handles {
        if let Some((boundary, padding_left, scale)) = boundaries.get(&child_of.parent()) {
            offsets.push((handle, true, (boundary - padding_left) / scale));
        }
    }
    for leading in &q_leading {
        if let Some((boundary, _, scale)) = q_parents
            .iter_ancestors(leading)
            .find_map(|entity| boundaries.get(&entity))
            && let Some([start, _]) = border_box(leading, &geometry, *scale)
        {
            offsets.push((leading, false, (boundary - start) / scale));
        }
    }

    let mut q_nodes = nodes.p1();
    for (entity, is_handle, offset) in offsets {
        let value = px((offset - 0.5 * HANDLE_WIDTH).max(0.0));
        let Ok(mut node) = q_nodes.get_mut(entity) else {
            continue;
        };
        if is_handle && node.left != value {
            node.left = value;
        } else if !is_handle && node.width != value {
            node.width = value;
            node.min_width = value;
        }
    }
}

type Splits<'a> = (
    &'a mut ColumnSplit,
    &'a mut ColumnSplitDragState,
    &'a ComputedNode,
    Has<InteractionDisabled>,
);

fn column_split_on_drag_start(
    mut drag_start: On<PointerDragStart>,
    q_handles: Query<&ChildOf, With<ColumnSplitHandle>>,
    mut q_splits: Query<Splits>,
    mut capture_map: ResMut<PointerCaptureMap>,
) {
    if drag_start.button == PointerButton::Primary
        && let Ok(child_of) = q_handles.get(drag_start.entity)
        && let Ok((split, mut state, computed, false)) = q_splits.get_mut(child_of.parent())
    {
        drag_start.propagate(false);
        *state = ColumnSplitDragState {
            dragging: true,
            start: split.clamped(),
            width: computed.content_box().width() * computed.inverse_scale_factor,
        };
        capture_map.capture(
            drag_start.pointer.id,
            drag_start.entity,
            drag_start.hit.clone(),
        );
    }
}

fn column_split_on_drag(
    mut drag: On<PointerDrag>,
    q_handles: Query<&ChildOf, With<ColumnSplitHandle>>,
    mut q_splits: Query<Splits>,
    ui_scale: Res<UiScale>,
) {
    if drag.button == PointerButton::Primary
        && let Ok(child_of) = q_handles.get(drag.entity)
        && let Ok((mut split, state, _, disabled)) = q_splits.get_mut(child_of.parent())
        && state.dragging
    {
        drag.propagate(false);
        if !disabled && state.width > 0.0 {
            let fraction = state.start + drag.distance.x / ui_scale.0 / state.width;
            split.fraction = fraction.min(split.max).max(split.min);
        }
    }
}

fn column_split_on_drag_end(
    mut drag_end: On<PointerDragEnd>,
    q_handles: Query<&ChildOf, With<ColumnSplitHandle>>,
    mut q_splits: Query<Splits>,
) {
    if drag_end.button == PointerButton::Primary
        && let Ok(child_of) = q_handles.get(drag_end.entity)
        && let Ok((_, mut state, ..)) = q_splits.get_mut(child_of.parent())
        && state.dragging
    {
        drag_end.propagate(false);
        state.dragging = false;
    }
}

fn column_split_on_cancel(
    mut cancel: On<PointerCancel>,
    q_handles: Query<&ChildOf, With<ColumnSplitHandle>>,
    mut q_splits: Query<Splits>,
) {
    if let Ok(child_of) = q_handles.get(cancel.entity)
        && let Ok((mut split, mut state, ..)) = q_splits.get_mut(child_of.parent())
        && state.dragging
    {
        cancel.propagate(false);
        split.fraction = state.start;
        state.dragging = false;
    }
}

fn update_handle_styles(
    q_handles: Query<
        (Entity, &ChildOf, &Hovered, &Children, &EntityCursor),
        With<ColumnSplitHandle>,
    >,
    q_splits: Query<(&ColumnSplitDragState, Has<InteractionDisabled>)>,
    q_lines: Query<&ThemeBackgroundColor, With<ColumnSplitHandleLine>>,
    mut commands: Commands,
) {
    for (handle, child_of, hovered, children, cursor) in &q_handles {
        let Ok((state, disabled)) = q_splits.get(child_of.parent()) else {
            continue;
        };
        let icon = match disabled {
            true => SystemCursorIcon::NotAllowed,
            false => SystemCursorIcon::ColResize,
        };
        if *cursor != EntityCursor::System(icon) {
            commands.entity(handle).insert(EntityCursor::System(icon));
        }
        let token = match (disabled, state.dragging, hovered.0) {
            (true, ..) => tokens::SPLIT_PANE_HANDLE_DISABLED,
            (false, true, _) => tokens::SPLIT_PANE_HANDLE_PRESSED,
            (false, false, true) => tokens::SPLIT_PANE_HANDLE_HOVER,
            (false, false, false) => tokens::SPLIT_PANE_HANDLE,
        };
        for &line in children {
            if q_lines.get(line).is_ok_and(|color| color.0 != token) {
                commands
                    .entity(line)
                    .insert(ThemeBackgroundColor(token.clone()));
            }
        }
    }
}

/// Plugin which spawns, positions and drags the handles of [`ColumnSplit`]s, and sizes
/// [`ColumnSplitLeading`] nodes.
///
/// Layout is updated in [`UiSystems::Content`], so a moved boundary is laid out in the same frame.
pub struct ColumnSplitPlugin;

impl Plugin for ColumnSplitPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_handle)
            .add_observer(column_split_on_drag_start)
            .add_observer(column_split_on_drag)
            .add_observer(column_split_on_drag_end)
            .add_observer(column_split_on_cancel)
            .add_systems(
                PostUpdate,
                (update_column_splits, update_handle_styles).in_set(UiSystems::Content),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::PreUpdate;
    use bevy_camera::NormalizedRenderTarget;
    use bevy_math::Vec2;
    use bevy_picking::{
        backend::{HitData, PointerHits},
        cursor::CursorIconPlugin,
        events::Pointer,
        hover::{generate_hovermap, HoverMap, PreviousHoverMap},
        pointer::{Location, PointerAction, PointerId, PointerInput, PointerMap},
        PickingSystems,
    };
    use bevy_ui::{prelude::BorderRect, UiRect};
    use bevy_window::{CursorIcon, Window};

    fn split_app() -> App {
        let mut app = App::new();
        app.add_plugins(ColumnSplitPlugin)
            .init_resource::<PointerCaptureMap>()
            .init_resource::<UiScale>();
        app.update();
        app
    }

    fn computed(width: f32, padding: f32) -> ComputedNode {
        ComputedNode {
            size: Vec2::new(width, 20.0),
            padding: BorderRect {
                min_inset: Vec2::new(padding, 0.0),
                max_inset: Vec2::ZERO,
            },
            inverse_scale_factor: 1.0,
            ..Default::default()
        }
    }

    /// Spawns a split 200 pixels wide, starting at x = 0, and returns it with its handle.
    fn spawn_split(app: &mut App) -> (Entity, Entity) {
        let split = app
            .world_mut()
            .spawn((
                ColumnSplit::default(),
                Node::default(),
                computed(200.0, 0.0),
                UiGlobalTransform::from_xy(100.0, 0.0),
            ))
            .id();
        app.world_mut().flush();
        let handle = app.world().get::<Children>(split).unwrap()[0];
        (split, handle)
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

    fn drag_start(app: &mut App, handle: Entity, button: PointerButton) {
        app.world_mut().trigger(PointerDragStart {
            entity: handle,
            pointer: pointer(),
            button,
            hit: hit(),
        });
    }

    fn drag_to(app: &mut App, handle: Entity, button: PointerButton, distance: f32) {
        let distance = Vec2::new(distance, 7.0);
        app.world_mut().trigger(PointerDrag {
            entity: handle,
            pointer: pointer(),
            button,
            distance,
            delta: distance,
        });
    }

    fn drag_end(app: &mut App, handle: Entity) {
        app.world_mut().trigger(PointerDragEnd {
            entity: handle,
            pointer: pointer(),
            button: PointerButton::Primary,
            distance: Vec2::ZERO,
        });
    }

    fn drag(app: &mut App, handle: Entity, distance: f32) {
        drag_start(app, handle, PointerButton::Primary);
        drag_to(app, handle, PointerButton::Primary, distance);
        drag_end(app, handle);
    }

    fn fraction(app: &App, split: Entity) -> f32 {
        app.world().get::<ColumnSplit>(split).unwrap().fraction
    }

    fn dragging(app: &App, split: Entity) -> bool {
        app.world()
            .get::<ColumnSplitDragState>(split)
            .unwrap()
            .dragging
    }

    #[test]
    fn spawns_one_handle_spanning_the_container() {
        let mut app = split_app();
        let (split, handle) = spawn_split(&mut app);
        assert_eq!(app.world().get::<Children>(split).unwrap().len(), 1);
        let node = app.world().get::<Node>(handle).unwrap();
        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!((node.top, node.bottom), (px(0), px(0)));
        assert_eq!(
            app.world().get::<EntityCursor>(handle),
            Some(&EntityCursor::System(SystemCursorIcon::ColResize))
        );
    }

    #[test]
    fn drag_moves_the_boundary() {
        let mut app = split_app();
        let (split, handle) = spawn_split(&mut app);

        drag_start(&mut app, handle, PointerButton::Primary);
        drag_to(&mut app, handle, PointerButton::Primary, 20.0);
        assert!((fraction(&app, split) - 0.5).abs() < 1e-5);
        assert!(dragging(&app, split));

        drag_to(&mut app, handle, PointerButton::Primary, -20.0);
        drag_end(&mut app, handle);
        assert!((fraction(&app, split) - 0.3).abs() < 1e-5);
        assert!(!dragging(&app, split));
    }

    fn pointer_input(app: &mut App, action: PointerAction) {
        let pointer = pointer();
        let location = Location {
            target: pointer.target,
            position: pointer.position,
        };
        app.world_mut()
            .write_message(PointerInput::new(PointerId::Mouse, location, action));
    }

    fn hover_row(app: &mut App, row: Entity) -> Option<CursorIcon> {
        app.world_mut()
            .write_message(PointerHits::new(PointerId::Mouse, vec![(row, hit())], 0.0));
        app.update();
        let mut windows = app
            .world_mut()
            .query_filtered::<&CursorIcon, With<Window>>();
        windows.single(app.world()).ok().cloned()
    }

    #[test]
    fn drag_keeps_the_resize_cursor() {
        let mut app = split_app();
        app.add_plugins(CursorIconPlugin)
            .add_message::<PointerHits>()
            .add_message::<PointerInput>()
            .init_resource::<HoverMap>()
            .init_resource::<PreviousHoverMap>()
            .init_resource::<PointerMap>()
            .add_systems(PreUpdate, generate_hovermap.before(PickingSystems::Last));
        app.world_mut().spawn(PointerId::Mouse);
        app.world_mut().spawn(Window::default());
        let (split, handle) = spawn_split(&mut app);
        let row = app
            .world_mut()
            .spawn((
                EntityCursor::System(SystemCursorIcon::NotAllowed),
                ChildOf(split),
            ))
            .id();
        let not_allowed = Some(CursorIcon::from(SystemCursorIcon::NotAllowed));
        let col_resize = Some(CursorIcon::from(SystemCursorIcon::ColResize));
        assert_eq!(hover_row(&mut app, row), not_allowed);

        drag_start(&mut app, handle, PointerButton::Primary);
        drag_to(&mut app, handle, PointerButton::Primary, 20.0);
        assert_eq!(hover_row(&mut app, row), col_resize);

        drag_end(&mut app, handle);
        pointer_input(&mut app, PointerAction::Release(PointerButton::Primary));
        assert_eq!(hover_row(&mut app, row), not_allowed);

        drag_start(&mut app, handle, PointerButton::Primary);
        assert_eq!(hover_row(&mut app, row), col_resize);
        app.world_mut().trigger(PointerCancel {
            entity: handle,
            pointer: pointer(),
            hit: hit(),
        });
        pointer_input(&mut app, PointerAction::Cancel);
        hover_row(&mut app, row);
        assert!(!app
            .world()
            .resource::<PointerCaptureMap>()
            .is_captured(&PointerId::Mouse));
        assert_eq!(hover_row(&mut app, row), not_allowed);
    }

    #[test]
    fn drag_only_moves_its_own_split() {
        let mut app = split_app();
        let (first, handle) = spawn_split(&mut app);
        let (second, _) = spawn_split(&mut app);
        drag(&mut app, handle, 40.0);
        assert!((fraction(&app, first) - 0.6).abs() < 1e-5);
        assert_eq!(fraction(&app, second), 0.4);
    }

    #[test]
    fn drag_respects_ui_scale() {
        let mut app = split_app();
        app.insert_resource(UiScale(2.0));
        let (split, handle) = spawn_split(&mut app);
        drag(&mut app, handle, 40.0);
        assert!((fraction(&app, split) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn drag_clamps_to_min_and_max() {
        let mut app = split_app();
        let (split, handle) = spawn_split(&mut app);
        drag(&mut app, handle, 1000.0);
        assert_eq!(fraction(&app, split), 0.75);
        drag(&mut app, handle, -1000.0);
        assert_eq!(fraction(&app, split), 0.15);
    }

    #[test]
    fn cancel_restores_the_start_value() {
        let mut app = split_app();
        let (split, handle) = spawn_split(&mut app);
        drag_start(&mut app, handle, PointerButton::Primary);
        drag_to(&mut app, handle, PointerButton::Primary, 50.0);
        app.world_mut().trigger(PointerCancel {
            entity: handle,
            pointer: pointer(),
            hit: hit(),
        });
        assert_eq!(fraction(&app, split), 0.4);
        assert!(!dragging(&app, split));
    }

    #[test]
    fn disabled_split_ignores_drag() {
        let mut app = split_app();
        let (split, handle) = spawn_split(&mut app);
        app.world_mut()
            .entity_mut(split)
            .insert(InteractionDisabled);
        drag(&mut app, handle, 50.0);
        assert_eq!(fraction(&app, split), 0.4);
    }

    #[test]
    fn secondary_button_ignores_drag() {
        let mut app = split_app();
        let (split, handle) = spawn_split(&mut app);
        drag_start(&mut app, handle, PointerButton::Secondary);
        drag_to(&mut app, handle, PointerButton::Secondary, 50.0);
        assert_eq!(fraction(&app, split), 0.4);
        assert!(!dragging(&app, split));
    }

    /// Spawns a split from x = 0 to 200 with laid out rows indented by 0, 12 and 24 pixels, and
    /// returns the split's handle and the leading column of each row.
    fn spawn_rows(app: &mut App) -> (Entity, Vec<Entity>) {
        let (split, handle) = spawn_split(app);
        app.world_mut()
            .get_mut::<ColumnSplit>(split)
            .unwrap()
            .fraction = 0.5;
        let mut leading = Vec::new();
        for indent in [0.0, 12.0, 24.0] {
            let world = app.world_mut();
            let row = world
                .spawn((
                    Node {
                        padding: UiRect::left(px(indent)),
                        ..Default::default()
                    },
                    computed(200.0, indent),
                    UiGlobalTransform::from_xy(100.0, 0.0),
                    ChildOf(split),
                ))
                .id();
            let column = (
                ColumnSplitLeading,
                computed(40.0, 0.0),
                UiGlobalTransform::from_xy(indent + 20.0, 0.0),
                ChildOf(row),
            );
            leading.push(world.spawn(column).id());
        }
        (handle, leading)
    }

    fn widths(app: &App, nodes: &[Entity]) -> Vec<Val> {
        nodes
            .iter()
            .map(|&entity| app.world().get::<Node>(entity).unwrap().width)
            .collect()
    }

    #[test]
    fn leading_columns_end_at_the_handle() {
        let mut app = split_app();
        let (handle, leading) = spawn_rows(&mut app);
        app.update();
        app.update();
        assert_eq!(widths(&app, &leading), vec![px(97.5), px(85.5), px(73.5)]);
        assert_eq!(app.world().get::<Node>(handle).unwrap().left, px(97.5));
    }

    #[test]
    fn new_nodes_are_placed_from_their_ancestors() {
        let mut app = split_app();
        let (_, leading) = spawn_rows(&mut app);
        app.update();
        let row = app.world().get::<ChildOf>(leading[2]).unwrap().parent();
        let world = app.world_mut();
        let fresh = world
            .spawn((
                Node {
                    margin: UiRect::left(px(4.0)),
                    ..Default::default()
                },
                ChildOf(row),
            ))
            .id();
        let nested = world.spawn((ColumnSplitLeading, ChildOf(fresh))).id();
        app.update();
        assert_eq!(widths(&app, &[nested]), vec![px(69.5)]);
    }

    #[test]
    fn new_splits_stretch_across_their_parent() {
        let mut app = split_app();
        let parent = app
            .world_mut()
            .spawn((
                Node::default(),
                computed(200.0, 10.0),
                UiGlobalTransform::from_xy(100.0, 0.0),
            ))
            .id();
        app.update();
        let world = app.world_mut();
        let split = world.spawn((ColumnSplit::default(), ChildOf(parent))).id();
        let leading = world.spawn((ColumnSplitLeading, ChildOf(split))).id();
        app.update();
        assert_eq!(widths(&app, &[leading]), vec![px(73.5)]);
    }

    #[test]
    fn clamps_the_boundary() {
        let split = ColumnSplit {
            fraction: 0.9,
            ..Default::default()
        };
        assert_eq!(split.clamped(), 0.75);
    }
}
