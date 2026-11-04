use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Changed, Has, Or, With},
    reflect::{ReflectComponent, ReflectEvent},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut, SystemParam},
    template::FromTemplate,
};
use bevy_input::{
    keyboard::{KeyCode, KeyboardInput},
    ButtonState,
};
use bevy_input_focus::{
    tab_navigation::TabIndex, FocusCause, FocusedInput, InputFocus, InputFocusSystems,
    InputFocusVisible,
};
use bevy_math::Vec2;
use bevy_picking::{
    events::{
        PointerCancel, PointerClick, PointerDrag, PointerDragEnd, PointerDragStart, PointerState,
    },
    hover::HoverMap,
    pointer::{PointerButton, PointerId},
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{
    ComputedNode, InteractionDisabled, Selectable, Selected, UiGlobalTransform, UiScale,
};

use crate::ControlOrientation;

/// Determines whether moving keyboard focus also requests tab selection.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default, Clone, PartialEq)]
pub enum TabActivation {
    /// Enter or Space requests selection of the focused tab.
    #[default]
    Manual,
    /// Moving focus with a navigation key requests selection of the focused tab.
    Automatic,
}

/// Determines which drag gestures a [`TabList`] accepts.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default, Clone, PartialEq)]
pub enum TabDragMode {
    /// Dragging does not start a tab drag.
    #[default]
    Disabled,
    /// Tabs may be reordered within this list.
    ///
    /// Once a drag starts, the pointer's position along the list's axis picks the insertion
    /// point, even when the pointer leaves the list.
    Reorder,
    /// Tabs may be reordered within this list or moved between lists whose drag mode is also
    /// `External`.
    ///
    /// The target is the nearest such list under the pointer. Over no such list, including this
    /// one, there is no preview and the drag completes without a [`TabDrop`].
    External,
}

/// Headless tab-strip behavior and policy.
///
/// Selection is stored separately in [`SelectedTab`]. User interaction emits
/// [`crate::ValueChange<Option<Entity>>`] from this entity and does not update that state unless
/// [`tablist_self_update`] is attached as an observer. Activating the already-selected tab does
/// not re-emit.
///
/// Only primary-button clicks change selection. [`InteractionDisabled`] on this entity disables
/// the whole strip; disabled strips and disabled tabs let pointer and keyboard events propagate
/// instead of consuming them.
///
/// When [`TabList::drag`] allows it, dragging a tab emits [`TabMoved`] on release and does not
/// change the hierarchy itself. [`TabDragLifecycle`] reports each accepted drag.
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[require(AccessibilityNode(accesskit::Node::new(Role::TabList)), SelectedTab)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct TabList {
    /// The axis used by arrow-key navigation.
    pub orientation: ControlOrientation,
    /// Whether keyboard navigation requests selection immediately.
    pub activation: TabActivation,
    /// The drag gestures accepted by this list.
    pub drag: TabDragMode,
}

impl Default for TabList {
    fn default() -> Self {
        Self {
            orientation: ControlOrientation::Horizontal,
            activation: TabActivation::default(),
            drag: TabDragMode::default(),
        }
    }
}

/// The selected [`Tab`] within a [`TabList`].
///
/// The referenced entity must be an enabled direct child of the list. Missing, stale, disabled,
/// and unrelated entities are treated as no selection.
#[derive(Component, FromTemplate, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, PartialEq)]
pub struct SelectedTab(#[template(built_in)] pub Option<Entity>);

/// A headless tab header.
///
/// Tabs are focusable using a roving [`TabIndex`]. Their derived [`Selected`] state mirrors the
/// containing list's valid [`SelectedTab`] value.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[require(
    AccessibilityNode(accesskit::Node::new(Role::Tab)),
    Selectable,
    TabIndex(-1)
)]
#[reflect(Component, Default, Clone)]
pub struct Tab;

/// Prevents a [`Tab`] from starting a drag without disabling focus or activation.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct TabLocked;

/// Marks a tab whose drag has crossed the movement threshold.
///
/// Removed when the drag completes or is cancelled.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Clone, PartialEq)]
pub struct TabDragging {
    /// The pointer controlling the drag.
    pub pointer_id: PointerId,
}

/// One pointer's proposed insertion point within a [`TabList`].
///
/// `index` counts the list's tabs after removing `tab`, while `slot` counts all of its tabs. The
/// gaps on either side of `tab` share an `index` but not a `slot`, so styling can mark the side
/// nearest the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct TabInsertionPoint {
    /// The pointer controlling this preview.
    pub pointer_id: PointerId,
    /// The tab being dragged.
    pub tab: Entity,
    /// The proposed insertion index.
    pub index: usize,
    /// The gap to mark, counted in the list's current tab order.
    pub slot: usize,
}

/// The insertion points currently proposed on a [`TabList`].
///
/// Present only while at least one accepted drag targets the list, with at most one entry per
/// pointer.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct TabInsertionPreview {
    /// Proposed insertions, one for each controlling pointer.
    pub entries: Vec<TabInsertionPoint>,
}

/// Proposes moving a tab without changing the entity hierarchy.
///
/// `index` counts the destination's tabs after removing `tab`, so an observer can apply the move
/// with [`insert_child`](bevy_ecs::system::EntityCommands::insert_child) when the list contains
/// only tabs. When `to_strip` differs from `from_strip`, the same call moves the tab to the
/// destination list.
#[derive(Copy, Clone, Debug, PartialEq, Eq, EntityEvent, Reflect)]
#[reflect(Event, Clone, PartialEq)]
pub struct TabMoved {
    /// The list that currently contains the tab and receives this event.
    #[event_target]
    pub from_strip: Entity,
    /// The tab to move.
    pub tab: Entity,
    /// The destination list.
    pub to_strip: Entity,
    /// The proposed insertion index.
    pub index: usize,
}

/// A list and insertion index that accepted a dropped tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct TabDrop {
    /// The destination list.
    pub destination: Entity,
    /// The insertion index, counted after removing the dragged tab.
    pub index: usize,
}

/// A transition in the life of an accepted tab drag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Clone, PartialEq)]
pub enum TabDragPhase {
    /// The drag crossed the movement threshold.
    Started,
    /// The pointer was released.
    Completed {
        /// The accepting list, or `None` when released outside any compatible list.
        drop: Option<TabDrop>,
    },
    /// The drag was cancelled by Escape, pointer cancellation, or the tab despawning.
    Cancelled,
}

/// Reports the start and end of an accepted tab drag.
///
/// Every [`TabDragPhase::Started`] is followed by exactly one `Completed` or `Cancelled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EntityEvent, Reflect)]
#[reflect(Event, Clone, PartialEq)]
pub struct TabDragLifecycle {
    /// The list the drag started from, which receives this event.
    #[event_target]
    pub source: Entity,
    /// The dragged tab.
    pub tab: Entity,
    /// The pointer controlling the drag.
    pub pointer_id: PointerId,
    /// The transition being reported.
    pub phase: TabDragPhase,
}

const TAB_DRAG_THRESHOLD: f32 = 4.0;

#[derive(Resource, Default)]
struct TabDragGestures(Vec<TabDragGesture>);

struct TabDragGesture {
    pointer_id: PointerId,
    tab: Entity,
    source: Entity,
    state: TabDragState,
    preview: Option<TabDropTarget>,
}

#[derive(Clone, Copy)]
struct TabDropTarget {
    list: Entity,
    index: usize,
    slot: usize,
}

impl TabDropTarget {
    fn drop(self) -> TabDrop {
        TabDrop {
            destination: self.list,
            index: self.index,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TabDragState {
    Pending,
    Accepted,
    Cancelled,
}

impl TabDragGestures {
    fn position(&self, pointer_id: PointerId, tab: Entity) -> Option<usize> {
        self.0
            .iter()
            .position(|gesture| gesture.pointer_id == pointer_id && gesture.tab == tab)
    }
}

/// Plugin that registers tab-list observers and derived state.
pub struct TabPlugin;

impl Plugin for TabPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TabDragGestures>()
            .init_resource::<HoverMap>()
            .init_resource::<UiScale>()
            .add_observer(tablist_on_click)
            .add_observer(tab_on_key_input)
            .add_observer(tab_on_drag_start)
            .add_observer(tab_on_drag)
            .add_observer(tab_on_drag_end)
            .add_observer(tab_on_pointer_cancel)
            .add_observer(cancel_tab_drags_on_escape)
            .add_systems(
                PostUpdate,
                (
                    cleanup_tab_drags,
                    sync_tab_insertion_previews,
                    update_tablist_derived_state,
                )
                    .chain()
                    .after(crate::MenuFocusSystem)
                    .before(InputFocusSystems::FocusChangeEvents),
            );
    }
}

/// Observer that applies tab selection requests to [`SelectedTab`].
pub fn tablist_self_update(
    change: On<crate::ValueChange<Option<Entity>>>,
    tablists: Query<(), With<TabList>>,
    mut commands: Commands,
) {
    if tablists.contains(change.source) {
        commands
            .entity(change.source)
            .insert(SelectedTab(change.value));
    }
}

fn tablist_on_click(
    mut click: On<PointerClick>,
    tablists: Query<(&SelectedTab, Has<InteractionDisabled>), With<TabList>>,
    tabs: Query<Has<InteractionDisabled>, With<Tab>>,
    parents: Query<&ChildOf>,
    gestures: Res<TabDragGestures>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    let Ok((selection, list_disabled)) = tablists.get(click.entity) else {
        return;
    };

    let target = click.original_event_target();
    let tab = if tabs.contains(target) {
        Some(target)
    } else {
        parents
            .iter_ancestors(target)
            .take_while(|ancestor| *ancestor != click.entity)
            .find(|ancestor| tabs.contains(*ancestor))
    };
    let Some(tab) = tab else {
        return;
    };
    let Ok(parent) = parents.get(tab) else {
        return;
    };
    if parent.parent() != click.entity {
        return;
    }
    if list_disabled || tabs.get(tab).is_ok_and(|disabled| disabled) {
        return;
    }

    click.propagate(false);
    let dragged = gestures
        .position(click.pointer.id, tab)
        .is_some_and(|index| gestures.0[index].state != TabDragState::Pending);
    if !dragged && selection.0 != Some(tab) {
        commands.trigger(crate::ValueChange::<Option<Entity>> {
            source: click.entity,
            value: Some(tab),
            is_final: true,
        });
    }
}

fn tab_on_key_input(
    mut input: On<FocusedInput<KeyboardInput>>,
    tablists: Query<(&TabList, &SelectedTab, &Children, Has<InteractionDisabled>)>,
    tabs: Query<Has<InteractionDisabled>, With<Tab>>,
    parents: Query<&ChildOf>,
    mut focus: ResMut<InputFocus>,
    mut focus_visible: ResMut<InputFocusVisible>,
    mut commands: Commands,
) {
    if !tabs.contains(input.focused_entity) {
        return;
    }
    let Ok(parent) = parents.get(input.focused_entity) else {
        return;
    };
    let Ok((tablist, selection, children, list_disabled)) = tablists.get(parent.parent()) else {
        return;
    };
    if list_disabled {
        return;
    }
    let event = &input.input;
    if event.state != ButtonState::Pressed || event.repeat {
        return;
    }

    enum Navigation {
        Previous,
        Next,
        First,
        Last,
        Activate,
    }

    let navigation = match event.key_code {
        KeyCode::ArrowLeft if tablist.orientation == ControlOrientation::Horizontal => {
            Navigation::Previous
        }
        KeyCode::ArrowRight if tablist.orientation == ControlOrientation::Horizontal => {
            Navigation::Next
        }
        KeyCode::ArrowUp if tablist.orientation == ControlOrientation::Vertical => {
            Navigation::Previous
        }
        KeyCode::ArrowDown if tablist.orientation == ControlOrientation::Vertical => {
            Navigation::Next
        }
        KeyCode::Home => Navigation::First,
        KeyCode::End => Navigation::Last,
        KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => Navigation::Activate,
        _ => return,
    };

    if matches!(navigation, Navigation::Activate) {
        if tabs
            .get(input.focused_entity)
            .is_ok_and(|disabled| disabled)
        {
            return;
        }
        input.propagate(false);
        if selection.0 != Some(input.focused_entity) {
            commands.trigger(crate::ValueChange::<Option<Entity>> {
                source: parent.parent(),
                value: Some(input.focused_entity),
                is_final: true,
            });
        }
        return;
    }
    input.propagate(false);

    let enabled = children
        .iter()
        .copied()
        .filter(|child| tabs.get(*child).is_ok_and(|disabled| !disabled))
        .collect::<Vec<_>>();
    if enabled.is_empty() {
        return;
    }

    let current = enabled
        .iter()
        .position(|tab| *tab == input.focused_entity)
        .or_else(|| {
            selection
                .0
                .and_then(|selected| enabled.iter().position(|tab| *tab == selected))
        });
    let next_index = match navigation {
        Navigation::Previous => current
            .filter(|index| *index > 0)
            .map_or(enabled.len() - 1, |index| index - 1),
        Navigation::Next => current.map_or(0, |index| (index + 1) % enabled.len()),
        Navigation::First => 0,
        Navigation::Last => enabled.len() - 1,
        Navigation::Activate => unreachable!(),
    };
    let next = enabled[next_index];
    if focus.get() != Some(next) {
        focus.set(next, FocusCause::Navigated);
    }
    focus_visible.0 = true;
    if tablist.activation == TabActivation::Automatic && selection.0 != Some(next) {
        commands.trigger(crate::ValueChange::<Option<Entity>> {
            source: parent.parent(),
            value: Some(next),
            is_final: true,
        });
    }
}

fn tab_on_drag_start(
    mut event: On<PointerDragStart>,
    tabs: Query<(Has<InteractionDisabled>, Has<TabLocked>), With<Tab>>,
    tablists: Query<(&TabList, Has<InteractionDisabled>)>,
    parents: Query<&ChildOf>,
    mut gestures: ResMut<TabDragGestures>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok((disabled, locked)) = tabs.get(event.entity) else {
        return;
    };
    let Ok(parent) = parents.get(event.entity) else {
        return;
    };
    let Ok((tablist, list_disabled)) = tablists.get(parent.parent()) else {
        return;
    };
    if disabled
        || locked
        || list_disabled
        || tablist.drag == TabDragMode::Disabled
        || gestures
            .0
            .iter()
            .any(|gesture| gesture.pointer_id == event.pointer.id || gesture.tab == event.entity)
    {
        return;
    }

    event.propagate(false);
    gestures.0.push(TabDragGesture {
        pointer_id: event.pointer.id,
        tab: event.entity,
        source: parent.parent(),
        state: TabDragState::Pending,
        preview: None,
    });
}

fn tab_on_drag(
    mut event: On<PointerDrag>,
    mut gestures: ResMut<TabDragGestures>,
    targets: TabDropTargets,
    mut commands: Commands,
) {
    let Some(index) = gestures.position(event.pointer.id, event.entity) else {
        return;
    };
    event.propagate(false);
    let gesture = &mut gestures.0[index];
    match gesture.state {
        TabDragState::Cancelled => return,
        TabDragState::Pending
            if event.distance.length_squared() < TAB_DRAG_THRESHOLD * TAB_DRAG_THRESHOLD =>
        {
            return;
        }
        TabDragState::Pending => {
            gesture.state = TabDragState::Accepted;
            commands.entity(gesture.tab).insert(TabDragging {
                pointer_id: gesture.pointer_id,
            });
            commands.trigger(TabDragLifecycle {
                source: gesture.source,
                tab: gesture.tab,
                pointer_id: gesture.pointer_id,
                phase: TabDragPhase::Started,
            });
        }
        TabDragState::Accepted => {}
    }
    gesture.preview = targets.resolve(gesture, event.pointer.position);
}

fn tab_on_drag_end(
    mut event: On<PointerDragEnd>,
    mut gestures: ResMut<TabDragGestures>,
    targets: TabDropTargets,
    mut commands: Commands,
) {
    let Some(index) = gestures.position(event.pointer.id, event.entity) else {
        return;
    };
    event.propagate(false);
    let gesture = gestures.0.remove(index);
    if gesture.state != TabDragState::Accepted {
        return;
    }
    let drop = targets
        .resolve(&gesture, event.pointer.position)
        .map(TabDropTarget::drop);
    end_tab_drag(&gesture, TabDragPhase::Completed { drop }, &mut commands);
}

fn tab_on_pointer_cancel(
    event: On<PointerCancel>,
    mut gestures: ResMut<TabDragGestures>,
    mut commands: Commands,
) {
    let pointer_id = event.pointer.id;
    if !gestures
        .0
        .iter()
        .any(|gesture| gesture.pointer_id == pointer_id)
    {
        return;
    }
    gestures.0.retain(|gesture| {
        if gesture.pointer_id != pointer_id {
            return true;
        }
        end_tab_drag(gesture, TabDragPhase::Cancelled, &mut commands);
        false
    });
}

fn cancel_tab_drags_on_escape(
    mut event: On<FocusedInput<KeyboardInput>>,
    mut gestures: ResMut<TabDragGestures>,
    mut commands: Commands,
) {
    if event.input.state != ButtonState::Pressed
        || event.input.repeat
        || event.input.key_code != KeyCode::Escape
        || !gestures
            .0
            .iter()
            .any(|gesture| gesture.state == TabDragState::Accepted)
    {
        return;
    }
    event.propagate(false);
    for gesture in &mut gestures.0 {
        if gesture.state == TabDragState::Accepted {
            end_tab_drag(gesture, TabDragPhase::Cancelled, &mut commands);
            gesture.state = TabDragState::Cancelled;
            gesture.preview = None;
        }
    }
}

/// Ends an accepted drag, proposing a [`TabMoved`] for a completed drop.
fn end_tab_drag(gesture: &TabDragGesture, phase: TabDragPhase, commands: &mut Commands) {
    if gesture.state != TabDragState::Accepted {
        return;
    }
    if let Ok(mut tab) = commands.get_entity(gesture.tab) {
        tab.try_remove::<TabDragging>();
    }
    if let TabDragPhase::Completed { drop: Some(drop) } = phase {
        commands.trigger(TabMoved {
            from_strip: gesture.source,
            tab: gesture.tab,
            to_strip: drop.destination,
            index: drop.index,
        });
    }
    commands.trigger(TabDragLifecycle {
        source: gesture.source,
        tab: gesture.tab,
        pointer_id: gesture.pointer_id,
        phase,
    });
}

/// Drops gestures whose tab is gone or whose pointer is no longer dragging.
fn cleanup_tab_drags(
    mut gestures: ResMut<TabDragGestures>,
    tabs: Query<(), With<Tab>>,
    pointer_state: Option<Res<PointerState>>,
    mut commands: Commands,
) {
    if gestures.0.is_empty() {
        return;
    }
    gestures.0.retain(|gesture| {
        let dragging = pointer_state.as_ref().is_none_or(|pointer_state| {
            pointer_state
                .get(gesture.pointer_id, PointerButton::Primary)
                .is_some_and(|state| !state.dragging.is_empty())
        });
        if dragging && tabs.contains(gesture.tab) {
            return true;
        }
        end_tab_drag(gesture, TabDragPhase::Cancelled, &mut commands);
        false
    });
}

fn sync_tab_insertion_previews(
    gestures: Res<TabDragGestures>,
    mut tablists: Query<(Entity, Option<&mut TabInsertionPreview>), With<TabList>>,
    mut commands: Commands,
) {
    if !gestures.is_changed() {
        return;
    }
    for (list, preview) in &mut tablists {
        let entries = gestures
            .0
            .iter()
            .filter_map(|gesture| {
                let target = gesture.preview?;
                (target.list == list).then_some(TabInsertionPoint {
                    pointer_id: gesture.pointer_id,
                    tab: gesture.tab,
                    index: target.index,
                    slot: target.slot,
                })
            })
            .collect::<Vec<_>>();
        match preview {
            Some(_) if entries.is_empty() => {
                commands.entity(list).remove::<TabInsertionPreview>();
            }
            Some(mut preview) => {
                preview.set_if_neq(TabInsertionPreview { entries });
            }
            None if !entries.is_empty() => {
                commands
                    .entity(list)
                    .insert(TabInsertionPreview { entries });
            }
            None => {}
        }
    }
}

/// Resolves where a dragged tab would be inserted.
#[derive(SystemParam)]
struct TabDropTargets<'w, 's> {
    tablists: Query<'w, 's, (&'static TabList, Has<InteractionDisabled>)>,
    parents: Query<'w, 's, &'static ChildOf>,
    children: Query<'w, 's, &'static Children>,
    tabs: Query<'w, 's, Option<(&'static ComputedNode, &'static UiGlobalTransform)>, With<Tab>>,
    hover_map: Res<'w, HoverMap>,
    ui_scale: Res<'w, UiScale>,
}

impl TabDropTargets<'_, '_> {
    /// Returns where the dragged tab would land.
    ///
    /// A `Reorder` drag targets its source list wherever the pointer is. An `External` drag
    /// targets the nearest enabled `External` list under the pointer.
    fn resolve(&self, gesture: &TabDragGesture, position: Vec2) -> Option<TabDropTarget> {
        let (source, _) = self.tablists.get(gesture.source).ok()?;
        let destination = match source.drag {
            TabDragMode::Disabled => return None,
            TabDragMode::Reorder => gesture.source,
            TabDragMode::External => {
                self.hover_map
                    .get(&gesture.pointer_id)?
                    .iter()
                    .filter_map(|(entity, hit)| {
                        let list = self.nearest_tablist(*entity)?;
                        let (tablist, disabled) = self.tablists.get(list).ok()?;
                        (tablist.drag == TabDragMode::External && !disabled)
                            .then_some((list, hit.depth))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1))?
                    .0
            }
        };
        let (tablist, disabled) = self.tablists.get(destination).ok()?;
        if disabled {
            return None;
        }
        self.insertion_point(destination, tablist, gesture.tab, position)
    }

    fn nearest_tablist(&self, entity: Entity) -> Option<Entity> {
        if self.tablists.contains(entity) {
            return Some(entity);
        }
        self.parents
            .iter_ancestors(entity)
            .find(|ancestor| self.tablists.contains(*ancestor))
    }

    /// Counts the tabs whose center lies before `position` along the list's axis, with and
    /// without `dragged`.
    fn insertion_point(
        &self,
        list: Entity,
        tablist: &TabList,
        dragged: Entity,
        position: Vec2,
    ) -> Option<TabDropTarget> {
        let axis = |point: Vec2| match tablist.orientation {
            ControlOrientation::Horizontal => point.x,
            ControlOrientation::Vertical => point.y,
        };
        let position = axis(position / self.ui_scale.0.max(f32::EPSILON));
        let mut target = TabDropTarget {
            list,
            index: 0,
            slot: 0,
        };
        if let Ok(children) = self.children.get(list) {
            for child in children.iter().copied() {
                let Ok(geometry) = self.tabs.get(child) else {
                    continue;
                };
                let (node, transform) = geometry?;
                if position < axis(transform.translation * node.inverse_scale_factor) {
                    break;
                }
                target.slot += 1;
                if child != dragged {
                    target.index += 1;
                }
            }
        }
        Some(target)
    }
}

/// Derives per-tab state from each [`TabList`]'s [`SelectedTab`] and the current keyboard focus:
///
/// - [`Selected`] markers mirror a validated `SelectedTab` (the referenced entity must be an
///   enabled direct child; anything else counts as no selection).
/// - [`TabIndex`] follows the roving-tabindex pattern: one tab per list is sequentially reachable (the
///   focused tab, else the selected tab, else the first enabled tab), so Tab/Shift+Tab skip
///   the strip while arrow keys move within it.
///
/// Runs in `PostUpdate`, early-returning unless selection, children, focus, or disabled state
/// changed.
fn update_tablist_derived_state(
    tablists: Query<(&SelectedTab, &Children), With<TabList>>,
    tabs: Query<(Has<InteractionDisabled>, Has<Selected>, &TabIndex), With<Tab>>,
    focus: Option<Res<InputFocus>>,
    changed_tablists: Query<(), (With<TabList>, Or<(Changed<SelectedTab>, Changed<Children>)>)>,
    changed_tabs: Query<(), (With<Tab>, Or<(Added<Tab>, Added<InteractionDisabled>)>)>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    let focus_changed = focus.as_ref().is_some_and(DetectChanges::is_changed);
    let disabled_removed = !removed_disabled.is_empty();
    removed_disabled.clear();
    if !focus_changed && !disabled_removed && changed_tablists.is_empty() && changed_tabs.is_empty()
    {
        return;
    }

    for (selection, children) in tablists.iter() {
        let selected = selection.0.filter(|entity| {
            children.contains(entity) && tabs.get(*entity).is_ok_and(|(disabled, _, _)| !disabled)
        });
        let focused = focus
            .as_ref()
            .and_then(|focus| focus.get())
            .filter(|entity| {
                children.contains(entity)
                    && tabs.get(*entity).is_ok_and(|(disabled, _, _)| !disabled)
            });
        let roving = focused.or(selected).or_else(|| {
            children
                .iter()
                .find(|child| tabs.get(**child).is_ok_and(|(disabled, _, _)| !disabled))
                .copied()
        });

        for child in children.iter() {
            let Ok((_, is_selected, tab_index)) = tabs.get(*child) else {
                continue;
            };
            let should_select = selected == Some(*child);
            if should_select && !is_selected {
                commands.entity(*child).insert(Selected);
            } else if !should_select && is_selected {
                commands.entity(*child).remove::<Selected>();
            }

            let desired_index = if roving == Some(*child) { 0 } else { -1 };
            if tab_index.0 != desired_index {
                commands.entity(*child).insert(TabIndex(desired_index));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::{hierarchy::ChildOf, observer::On, resource::Resource, system::ResMut};
    use bevy_input::{keyboard::Key, InputPlugin};
    use bevy_input_focus::{FocusCause, InputDispatchPlugin, InputFocusPlugin};
    use bevy_math::Vec2;
    use bevy_picking::{
        backend::HitData,
        events::{DragEntry, Pointer},
        pointer::{Location, PointerButton, PointerId},
    };
    use bevy_window::{PrimaryWindow, Window, WindowRef};

    #[derive(Resource, Default)]
    struct SelectionRequests(Vec<(Entity, Option<Entity>)>);

    #[derive(Resource, Default)]
    struct TabMoveLog(Vec<TabMoved>);

    #[derive(Resource, Default)]
    struct DragLifecycleLog(Vec<TabDragLifecycle>);

    fn tab_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((
            InputPlugin,
            InputFocusPlugin,
            InputDispatchPlugin,
            TabPlugin,
        ))
        .init_resource::<SelectionRequests>()
        .init_resource::<TabMoveLog>()
        .init_resource::<DragLifecycleLog>()
        .add_observer(
            |change: On<crate::ValueChange<Option<Entity>>>,
             mut requests: ResMut<SelectionRequests>| {
                requests.0.push((change.source, change.value));
            },
        )
        .add_observer(|event: On<TabMoved>, mut log: ResMut<TabMoveLog>| {
            log.0.push(*event.event());
        })
        .add_observer(
            |event: On<TabDragLifecycle>, mut log: ResMut<DragLifecycleLog>| {
                log.0.push(*event.event());
            },
        );
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.update();
        (app, window)
    }

    fn press_key(app: &mut App, key_code: KeyCode, window: Entity) {
        let logical_key = match key_code {
            KeyCode::ArrowLeft => Key::ArrowLeft,
            KeyCode::ArrowRight => Key::ArrowRight,
            KeyCode::ArrowUp => Key::ArrowUp,
            KeyCode::ArrowDown => Key::ArrowDown,
            KeyCode::Home => Key::Home,
            KeyCode::End => Key::End,
            KeyCode::Enter | KeyCode::NumpadEnter => Key::Enter,
            KeyCode::Space => Key::Space,
            KeyCode::Escape => Key::Escape,
            _ => Key::Unidentified(bevy_input::keyboard::NativeKey::Unidentified),
        };
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }

    fn click(app: &mut App, target: Entity, window: Entity) {
        click_with_button(app, target, window, PointerButton::Primary);
    }

    fn window_location(window: Entity, position: Vec2) -> Location {
        Location {
            target: bevy_camera::NormalizedRenderTarget::Window(
                WindowRef::Entity(window).normalize(Some(window)).unwrap(),
            ),
            position,
        }
    }

    fn click_with_button(app: &mut App, target: Entity, window: Entity, button: PointerButton) {
        trigger_click(app, target, window, button);
        app.update();
    }

    fn trigger_click(app: &mut App, target: Entity, window: Entity, button: PointerButton) {
        app.world_mut().trigger(PointerClick {
            entity: target,
            pointer: Pointer::new(PointerId::Mouse, window_location(window, Vec2::ZERO)),
            button,
            hit: HitData::new(window, 0.0, None, None),
            duration: core::time::Duration::from_millis(10),
            count: 1,
        });
    }

    fn reorder_list(app: &mut App, window: Entity) -> Entity {
        drag_list(app, window, TabDragMode::Reorder)
    }

    fn external_list(app: &mut App, window: Entity) -> Entity {
        drag_list(app, window, TabDragMode::External)
    }

    fn drag_list(app: &mut App, window: Entity, drag: TabDragMode) -> Entity {
        app.world_mut()
            .spawn((
                TabList {
                    drag,
                    ..Default::default()
                },
                ChildOf(window),
            ))
            .id()
    }

    fn placed_tab(app: &mut App, list: Entity, x: f32) -> Entity {
        app.world_mut()
            .spawn((
                Tab,
                ComputedNode::default(),
                UiGlobalTransform::from_xy(x, 20.0),
                ChildOf(list),
            ))
            .id()
    }

    fn start_drag(app: &mut App, target: Entity, window: Entity, pointer_id: PointerId) {
        start_drag_at(app, target, window, pointer_id, Vec2::ZERO);
    }

    fn hover(app: &mut App, pointer_id: PointerId, entity: Entity, window: Entity) {
        hover_at_depth(app, pointer_id, entity, window, 0.0);
    }

    fn hover_at_depth(
        app: &mut App,
        pointer_id: PointerId,
        entity: Entity,
        window: Entity,
        depth: f32,
    ) {
        app.world_mut()
            .resource_mut::<HoverMap>()
            .entry(pointer_id)
            .or_default()
            .insert(entity, HitData::new(window, depth, None, None));
    }

    fn hover_only(app: &mut App, entity: Entity, window: Entity) {
        app.world_mut().resource_mut::<HoverMap>().clear();
        hover(app, PointerId::Mouse, entity, window);
    }

    fn phases(app: &App) -> Vec<TabDragPhase> {
        app.world()
            .resource::<DragLifecycleLog>()
            .0
            .iter()
            .map(|event| event.phase)
            .collect()
    }

    fn start_drag_at(
        app: &mut App,
        target: Entity,
        window: Entity,
        pointer_id: PointerId,
        start: Vec2,
    ) {
        app.world_mut().trigger(PointerDragStart {
            entity: target,
            pointer: Pointer::new(pointer_id, window_location(window, start)),
            button: PointerButton::Primary,
            hit: HitData::new(window, 0.0, None, None),
        });
        app.update();
    }

    fn drag_to(app: &mut App, target: Entity, window: Entity, pointer_id: PointerId, x: f32) {
        drag_between(
            app,
            target,
            window,
            pointer_id,
            Vec2::new(0.0, 20.0),
            Vec2::new(x, 20.0),
        );
    }

    fn drag_between(
        app: &mut App,
        target: Entity,
        window: Entity,
        pointer_id: PointerId,
        start: Vec2,
        position: Vec2,
    ) {
        app.world_mut().trigger(PointerDrag {
            entity: target,
            pointer: Pointer::new(pointer_id, window_location(window, position)),
            button: PointerButton::Primary,
            distance: position - start,
            delta: position - start,
        });
        app.update();
    }

    fn trigger_drag_end(app: &mut App, target: Entity, window: Entity, position: Vec2) {
        app.world_mut().trigger(PointerDragEnd {
            entity: target,
            pointer: Pointer::new(PointerId::Mouse, window_location(window, position)),
            button: PointerButton::Primary,
            distance: position,
        });
    }

    fn end_drag(app: &mut App, target: Entity, window: Entity, x: f32) {
        end_drag_at(app, target, window, Vec2::new(x, 20.0));
    }

    fn end_drag_at(app: &mut App, target: Entity, window: Entity, position: Vec2) {
        trigger_drag_end(app, target, window, position);
        app.update();
    }

    fn proposed(app: &App, list: Entity) -> Option<(usize, usize)> {
        preview(app, list).map(|entries| (entries[0].index, entries[0].slot))
    }

    fn moved_indices(app: &App) -> Vec<usize> {
        app.world()
            .resource::<TabMoveLog>()
            .0
            .iter()
            .map(|moved| moved.index)
            .collect()
    }

    fn preview(app: &App, list: Entity) -> Option<Vec<TabInsertionPoint>> {
        app.world()
            .entity(list)
            .get::<TabInsertionPreview>()
            .map(|preview| preview.entries.clone())
    }

    fn dragging(app: &App, tab: Entity) -> Option<PointerId> {
        app.world()
            .entity(tab)
            .get::<TabDragging>()
            .map(|dragging| dragging.pointer_id)
    }

    #[test]
    fn clicking_enabled_tab_requests_selection_without_mutating_controlled_state() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((TabList::default(), ChildOf(window)))
            .id();
        let first = app.world_mut().spawn((Tab, ChildOf(list))).id();
        let second = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.world_mut()
            .entity_mut(list)
            .insert(SelectedTab(Some(first)));
        app.update();

        click(&mut app, second, window);

        assert_eq!(
            app.world().resource::<SelectionRequests>().0,
            [(list, Some(second))]
        );
        assert_eq!(
            app.world().entity(list).get::<SelectedTab>(),
            Some(&SelectedTab(Some(first)))
        );
    }

    #[test]
    fn self_update_observer_applies_selection_request() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((TabList::default(), ChildOf(window)))
            .observe(tablist_self_update)
            .id();
        let tab = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.update();

        click(&mut app, tab, window);

        assert_eq!(
            app.world().entity(list).get::<SelectedTab>(),
            Some(&SelectedTab(Some(tab)))
        );
    }

    #[test]
    fn valid_selection_derives_selected_state_and_roving_entry() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((TabList::default(), ChildOf(window)))
            .id();
        let first = app.world_mut().spawn((Tab, ChildOf(list))).id();
        let second = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.world_mut()
            .entity_mut(list)
            .insert(SelectedTab(Some(second)));

        app.update();

        assert!(!app.world().entity(first).contains::<Selected>());
        assert!(app.world().entity(second).contains::<Selected>());
        assert_eq!(
            app.world().entity(first).get::<TabIndex>(),
            Some(&TabIndex(-1))
        );
        assert_eq!(
            app.world().entity(second).get::<TabIndex>(),
            Some(&TabIndex(0))
        );
    }

    #[test]
    fn invalid_selection_clears_selected_state_and_uses_first_enabled_roving_entry() {
        let (mut app, window) = tab_app();
        let stale = app.world_mut().spawn_empty().id();
        let list = app
            .world_mut()
            .spawn((
                TabList::default(),
                SelectedTab(Some(stale)),
                ChildOf(window),
            ))
            .id();
        let disabled = app
            .world_mut()
            .spawn((Tab, Selected, InteractionDisabled, ChildOf(list)))
            .id();
        let enabled = app.world_mut().spawn((Tab, ChildOf(list))).id();

        app.update();

        assert!(!app.world().entity(disabled).contains::<Selected>());
        assert!(!app.world().entity(enabled).contains::<Selected>());
        assert_eq!(
            app.world().entity(disabled).get::<TabIndex>(),
            Some(&TabIndex(-1))
        );
        assert_eq!(
            app.world().entity(enabled).get::<TabIndex>(),
            Some(&TabIndex(0))
        );
    }

    #[test]
    fn horizontal_manual_arrow_navigation_wraps_and_skips_disabled_tabs() {
        use bevy_input::keyboard::KeyCode;

        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((TabList::default(), ChildOf(window)))
            .id();
        let first = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.world_mut()
            .spawn((Tab, InteractionDisabled, ChildOf(list)));
        let last = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.world_mut()
            .entity_mut(list)
            .insert(SelectedTab(Some(first)));
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(first, FocusCause::Navigated);
        app.update();

        press_key(&mut app, KeyCode::ArrowRight, window);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(last));

        press_key(&mut app, KeyCode::ArrowRight, window);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(first));

        press_key(&mut app, KeyCode::ArrowLeft, window);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(last));
        assert!(
            app.world().resource::<SelectionRequests>().0.is_empty(),
            "manual navigation must not request selection"
        );
    }

    #[test]
    fn automatic_navigation_requests_selection_when_focus_moves() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((
                TabList {
                    activation: TabActivation::Automatic,
                    ..Default::default()
                },
                ChildOf(window),
            ))
            .id();
        let first = app.world_mut().spawn((Tab, ChildOf(list))).id();
        let second = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.world_mut()
            .entity_mut(list)
            .insert(SelectedTab(Some(first)));
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(first, FocusCause::Navigated);
        app.update();

        press_key(&mut app, KeyCode::ArrowRight, window);

        assert_eq!(app.world().resource::<InputFocus>().get(), Some(second));
        assert_eq!(
            app.world().resource::<SelectionRequests>().0,
            [(list, Some(second))]
        );
        assert_eq!(
            app.world().entity(list).get::<SelectedTab>(),
            Some(&SelectedTab(Some(first))),
            "automatic activation remains a controlled selection request"
        );
    }

    #[test]
    fn manual_tabs_request_selection_on_enter_and_space() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((TabList::default(), ChildOf(window)))
            .id();
        let tab = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(tab, FocusCause::Navigated);
        app.update();

        press_key(&mut app, KeyCode::Enter, window);
        press_key(&mut app, KeyCode::NumpadEnter, window);
        press_key(&mut app, KeyCode::Space, window);

        assert_eq!(
            app.world().resource::<SelectionRequests>().0,
            [(list, Some(tab)), (list, Some(tab)), (list, Some(tab))]
        );
    }

    #[test]
    fn vertical_tabs_use_up_and_down_arrows_only() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((
                TabList {
                    orientation: ControlOrientation::Vertical,
                    ..Default::default()
                },
                ChildOf(window),
            ))
            .id();
        let first = app.world_mut().spawn((Tab, ChildOf(list))).id();
        let second = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(first, FocusCause::Navigated);
        app.update();

        press_key(&mut app, KeyCode::ArrowRight, window);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(first));

        press_key(&mut app, KeyCode::ArrowDown, window);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(second));

        press_key(&mut app, KeyCode::ArrowUp, window);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(first));
    }

    #[test]
    fn home_and_end_focus_first_and_last_enabled_tabs() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((TabList::default(), ChildOf(window)))
            .id();
        app.world_mut()
            .spawn((Tab, InteractionDisabled, ChildOf(list)));
        let first_enabled = app.world_mut().spawn((Tab, ChildOf(list))).id();
        let last_enabled = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.world_mut()
            .spawn((Tab, InteractionDisabled, ChildOf(list)));
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(first_enabled, FocusCause::Navigated);
        app.update();

        press_key(&mut app, KeyCode::End, window);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(last_enabled)
        );

        press_key(&mut app, KeyCode::Home, window);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(first_enabled)
        );
    }

    #[test]
    fn focused_tab_is_the_roving_entry_during_manual_navigation() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((TabList::default(), ChildOf(window)))
            .id();
        let selected = app.world_mut().spawn((Tab, ChildOf(list))).id();
        let focused = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.world_mut()
            .entity_mut(list)
            .insert(SelectedTab(Some(selected)));
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(focused, FocusCause::Navigated);

        app.update();

        assert_eq!(
            app.world().entity(selected).get::<TabIndex>(),
            Some(&TabIndex(-1))
        );
        assert_eq!(
            app.world().entity(focused).get::<TabIndex>(),
            Some(&TabIndex(0))
        );
        assert!(app.world().entity(selected).contains::<Selected>());
        assert!(!app.world().entity(focused).contains::<Selected>());
    }

    #[test]
    fn tablist_and_tab_install_accessibility_semantics() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((TabList::default(), ChildOf(window)))
            .id();
        let tab = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.update();

        let list_node = app.world().entity(list).get::<AccessibilityNode>().unwrap();
        let tab_node = app.world().entity(tab).get::<AccessibilityNode>().unwrap();
        assert_eq!(list_node.role(), Role::TabList);
        assert_eq!(tab_node.role(), Role::Tab);
        assert!(app.world().entity(tab).contains::<Selectable>());
    }

    #[test]
    fn disabled_tab_does_not_request_selection() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((TabList::default(), ChildOf(window)))
            .id();
        let disabled = app
            .world_mut()
            .spawn((Tab, InteractionDisabled, ChildOf(list)))
            .id();
        app.update();

        click(&mut app, disabled, window);

        assert!(app.world().resource::<SelectionRequests>().0.is_empty());
    }

    #[test]
    fn secondary_click_does_not_change_selection() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((TabList::default(), ChildOf(window)))
            .id();
        let tab = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.update();

        click_with_button(&mut app, tab, window, PointerButton::Secondary);
        click_with_button(&mut app, tab, window, PointerButton::Middle);

        assert!(app.world().resource::<SelectionRequests>().0.is_empty());
    }

    #[test]
    fn activating_the_selected_tab_does_not_reemit() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((TabList::default(), ChildOf(window)))
            .observe(tablist_self_update)
            .id();
        let tab = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.update();

        click(&mut app, tab, window);
        click(&mut app, tab, window);

        assert_eq!(
            app.world().resource::<SelectionRequests>().0,
            [(list, Some(tab))]
        );
        assert_eq!(
            app.world().entity(list).get::<SelectedTab>(),
            Some(&SelectedTab(Some(tab)))
        );
    }

    #[test]
    fn disabled_tablist_ignores_clicks_and_keys() {
        let (mut app, window) = tab_app();
        let list = app
            .world_mut()
            .spawn((TabList::default(), InteractionDisabled, ChildOf(window)))
            .id();
        let first = app.world_mut().spawn((Tab, ChildOf(list))).id();
        let second = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(first, FocusCause::Navigated);
        app.update();

        click(&mut app, second, window);
        press_key(&mut app, KeyCode::ArrowRight, window);
        press_key(&mut app, KeyCode::Enter, window);

        assert!(app.world().resource::<SelectionRequests>().0.is_empty());
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(first));
    }

    #[test]
    fn tab_drag_is_accepted_only_after_crossing_movement_threshold() {
        let (mut app, window) = tab_app();
        let list = reorder_list(&mut app, window);
        let tab = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.update();

        start_drag(&mut app, tab, window, PointerId::Mouse);
        drag_to(&mut app, tab, window, PointerId::Mouse, 3.0);
        assert_eq!(dragging(&app, tab), None);

        drag_to(&mut app, tab, window, PointerId::Mouse, 4.0);
        assert_eq!(dragging(&app, tab), Some(PointerId::Mouse));
    }

    #[test]
    fn disabled_mode_locked_tabs_and_disabled_lists_do_not_drag() {
        let (mut app, window) = tab_app();
        let disabled_mode = app
            .world_mut()
            .spawn((TabList::default(), ChildOf(window)))
            .id();
        let disabled_mode_tab = app.world_mut().spawn((Tab, ChildOf(disabled_mode))).id();
        let reorder = reorder_list(&mut app, window);
        let locked_tab = app
            .world_mut()
            .spawn((Tab, TabLocked, ChildOf(reorder)))
            .id();
        let disabled_list = reorder_list(&mut app, window);
        app.world_mut()
            .entity_mut(disabled_list)
            .insert(InteractionDisabled);
        let disabled_list_tab = app.world_mut().spawn((Tab, ChildOf(disabled_list))).id();
        app.update();

        for (pointer_id, tab) in [
            (PointerId::Mouse, disabled_mode_tab),
            (PointerId::Touch(1), locked_tab),
            (PointerId::Touch(2), disabled_list_tab),
        ] {
            start_drag(&mut app, tab, window, pointer_id);
            drag_to(&mut app, tab, window, pointer_id, 20.0);
            assert_eq!(dragging(&app, tab), None);
        }
        assert!(app.world().entity(locked_tab).contains::<Selectable>());
    }

    #[test]
    fn drop_proposes_post_removal_index_without_mutating_hierarchy() {
        let (mut app, window) = tab_app();
        let list = reorder_list(&mut app, window);
        let first = placed_tab(&mut app, list, 50.0);
        let second = placed_tab(&mut app, list, 150.0);
        let third = placed_tab(&mut app, list, 250.0);
        app.update();
        let order = [first, second, third];

        start_drag(&mut app, first, window, PointerId::Mouse);
        drag_to(&mut app, first, window, PointerId::Mouse, 200.0);
        assert_eq!(
            preview(&app, list),
            Some(vec![TabInsertionPoint {
                pointer_id: PointerId::Mouse,
                tab: first,
                index: 1,
                slot: 2,
            }])
        );

        drag_to(&mut app, first, window, PointerId::Mouse, 400.0);
        assert_eq!(preview(&app, list).unwrap()[0].index, 2);

        end_drag(&mut app, first, window, 400.0);
        assert_eq!(
            app.world().resource::<TabMoveLog>().0,
            [TabMoved {
                from_strip: list,
                tab: first,
                to_strip: list,
                index: 2,
            }]
        );
        assert_eq!(
            app.world().entity(list).get::<Children>().unwrap().as_ref(),
            order
        );
        assert_eq!(preview(&app, list), None);
        assert_eq!(dragging(&app, first), None);
    }

    #[test]
    fn insertion_index_accounts_for_ui_scale() {
        let (mut app, window) = tab_app();
        app.insert_resource(UiScale(2.0));
        let list = reorder_list(&mut app, window);
        let first = placed_tab(&mut app, list, 50.0);
        placed_tab(&mut app, list, 150.0);
        app.update();

        start_drag(&mut app, first, window, PointerId::Mouse);
        drag_to(&mut app, first, window, PointerId::Mouse, 200.0);

        assert_eq!(preview(&app, list).unwrap()[0].index, 0);
    }

    #[test]
    fn reorder_keeps_tracking_with_the_pointer_above_or_below_the_strip() {
        let (mut app, window) = tab_app();
        let list = reorder_list(&mut app, window);
        let other = reorder_list(&mut app, window);
        let first = placed_tab(&mut app, list, 50.0);
        placed_tab(&mut app, list, 150.0);
        placed_tab(&mut app, list, 250.0);
        placed_tab(&mut app, other, 150.0);
        app.update();

        start_drag(&mut app, first, window, PointerId::Mouse);
        drag_between(
            &mut app,
            first,
            window,
            PointerId::Mouse,
            Vec2::ZERO,
            Vec2::new(200.0, -500.0),
        );
        assert_eq!(proposed(&app, list), Some((1, 2)));

        drag_between(
            &mut app,
            first,
            window,
            PointerId::Mouse,
            Vec2::ZERO,
            Vec2::new(400.0, 900.0),
        );
        assert_eq!(proposed(&app, list), Some((2, 3)));
        assert_eq!(preview(&app, other), None);

        end_drag_at(&mut app, first, window, Vec2::new(200.0, 900.0));
        assert_eq!(
            app.world().resource::<TabMoveLog>().0,
            [TabMoved {
                from_strip: list,
                tab: first,
                to_strip: list,
                index: 1,
            }]
        );
        assert_eq!(preview(&app, list), None);
    }

    #[test]
    fn reorder_clamps_past_the_ends_of_the_strip() {
        let (mut app, window) = tab_app();
        let list = reorder_list(&mut app, window);
        placed_tab(&mut app, list, 50.0);
        let middle = placed_tab(&mut app, list, 150.0);
        placed_tab(&mut app, list, 250.0);
        app.update();

        start_drag(&mut app, middle, window, PointerId::Mouse);
        drag_to(&mut app, middle, window, PointerId::Mouse, -1000.0);
        assert_eq!(proposed(&app, list), Some((0, 0)));

        drag_to(&mut app, middle, window, PointerId::Mouse, 5000.0);
        assert_eq!(proposed(&app, list), Some((2, 3)));

        end_drag(&mut app, middle, window, -1000.0);
        assert_eq!(moved_indices(&app), [0]);
    }

    #[test]
    fn insertion_slot_follows_the_pointer_either_side_of_the_dragged_tab() {
        let (mut app, window) = tab_app();
        let list = reorder_list(&mut app, window);
        placed_tab(&mut app, list, 50.0);
        let middle = placed_tab(&mut app, list, 150.0);
        let last = placed_tab(&mut app, list, 250.0);
        app.update();

        for (tab, center, index) in [(middle, 150.0, 1), (last, 250.0, 2)] {
            let start = Vec2::new(center, 20.0);
            start_drag_at(&mut app, tab, window, PointerId::Mouse, start);
            let before = start - Vec2::new(10.0, 0.0);
            drag_between(&mut app, tab, window, PointerId::Mouse, start, before);
            assert_eq!(proposed(&app, list), Some((index, index)));

            let after = start + Vec2::new(10.0, 0.0);
            drag_between(&mut app, tab, window, PointerId::Mouse, start, after);
            assert_eq!(proposed(&app, list), Some((index, index + 1)));

            end_drag(&mut app, tab, window, center + 10.0);
        }
        assert_eq!(moved_indices(&app), [1, 2]);
    }

    #[test]
    fn grab_offset_does_not_change_the_insertion_point() {
        let mut results = Vec::new();
        for grab in [105.0, 195.0] {
            let (mut app, window) = tab_app();
            let list = reorder_list(&mut app, window);
            placed_tab(&mut app, list, 50.0);
            let middle = placed_tab(&mut app, list, 150.0);
            placed_tab(&mut app, list, 250.0);
            app.update();

            let start = Vec2::new(grab, 20.0);
            start_drag_at(&mut app, middle, window, PointerId::Mouse, start);
            let mut points = Vec::new();
            for x in [40.0, 140.0, 160.0, 260.0] {
                drag_between(
                    &mut app,
                    middle,
                    window,
                    PointerId::Mouse,
                    start,
                    Vec2::new(x, 20.0),
                );
                points.push(proposed(&app, list));
            }
            end_drag(&mut app, middle, window, 140.0);
            results.push((points, moved_indices(&app)));
        }

        let expected = vec![Some((0, 0)), Some((1, 1)), Some((1, 2)), Some((2, 3))];
        assert_eq!(results, [(expected.clone(), vec![1]), (expected, vec![1])]);
    }

    #[test]
    fn previews_are_tracked_per_pointer() {
        let (mut app, window) = tab_app();
        let list = reorder_list(&mut app, window);
        let mouse_tab = placed_tab(&mut app, list, 50.0);
        let touch_tab = placed_tab(&mut app, list, 150.0);
        app.update();

        start_drag(&mut app, mouse_tab, window, PointerId::Mouse);
        drag_to(&mut app, mouse_tab, window, PointerId::Mouse, 200.0);
        start_drag(&mut app, touch_tab, window, PointerId::Touch(1));
        drag_to(&mut app, touch_tab, window, PointerId::Touch(1), 20.0);
        assert_eq!(preview(&app, list).map(|entries| entries.len()), Some(2));

        app.world_mut().trigger(PointerCancel {
            entity: list,
            pointer: Pointer::new(PointerId::Touch(1), window_location(window, Vec2::ZERO)),
            hit: HitData::new(window, 0.0, None, None),
        });
        app.update();

        assert_eq!(
            preview(&app, list),
            Some(vec![TabInsertionPoint {
                pointer_id: PointerId::Mouse,
                tab: mouse_tab,
                index: 1,
                slot: 2,
            }])
        );
        assert_eq!(dragging(&app, touch_tab), None);
        assert_eq!(dragging(&app, mouse_tab), Some(PointerId::Mouse));
    }

    #[test]
    fn the_same_tab_cannot_be_dragged_by_two_pointers() {
        let (mut app, window) = tab_app();
        let list = reorder_list(&mut app, window);
        let tab = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.update();

        start_drag(&mut app, tab, window, PointerId::Mouse);
        start_drag(&mut app, tab, window, PointerId::Touch(1));
        drag_to(&mut app, tab, window, PointerId::Touch(1), 20.0);
        assert_eq!(dragging(&app, tab), None);

        drag_to(&mut app, tab, window, PointerId::Mouse, 20.0);
        assert_eq!(dragging(&app, tab), Some(PointerId::Mouse));
    }

    #[test]
    fn escape_cancels_accepted_drags_until_release() {
        let (mut app, window) = tab_app();
        let list = reorder_list(&mut app, window);
        let first = placed_tab(&mut app, list, 50.0);
        let second = placed_tab(&mut app, list, 150.0);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(first, FocusCause::Navigated);
        app.update();

        start_drag(&mut app, second, window, PointerId::Mouse);
        drag_to(&mut app, second, window, PointerId::Mouse, 20.0);
        press_key(&mut app, KeyCode::Escape, window);
        assert_eq!(dragging(&app, second), None);
        assert_eq!(preview(&app, list), None);

        drag_to(&mut app, second, window, PointerId::Mouse, 30.0);
        assert_eq!(dragging(&app, second), None);
        assert_eq!(preview(&app, list), None);

        trigger_click(&mut app, second, window, PointerButton::Primary);
        end_drag(&mut app, second, window, 30.0);
        assert!(app.world().resource::<TabMoveLog>().0.is_empty());
        assert!(app.world().resource::<SelectionRequests>().0.is_empty());
    }

    #[test]
    fn despawning_the_dragged_tab_clears_the_preview() {
        let (mut app, window) = tab_app();
        let list = reorder_list(&mut app, window);
        let tab = placed_tab(&mut app, list, 50.0);
        app.update();

        start_drag(&mut app, tab, window, PointerId::Mouse);
        drag_to(&mut app, tab, window, PointerId::Mouse, 20.0);
        assert!(preview(&app, list).is_some());

        app.world_mut().despawn(tab);
        app.update();

        assert_eq!(preview(&app, list), None);
        let next = placed_tab(&mut app, list, 50.0);
        start_drag(&mut app, next, window, PointerId::Mouse);
        drag_to(&mut app, next, window, PointerId::Mouse, 20.0);
        assert_eq!(dragging(&app, next), Some(PointerId::Mouse));
    }

    #[test]
    fn gesture_is_dropped_when_the_pointer_stops_dragging() {
        let (mut app, window) = tab_app();
        app.init_resource::<PointerState>();
        let list = reorder_list(&mut app, window);
        let tab = app.world_mut().spawn((Tab, ChildOf(list))).id();
        app.world_mut()
            .resource_mut::<PointerState>()
            .get_mut(PointerId::Mouse, PointerButton::Primary)
            .dragging
            .insert(
                tab,
                DragEntry {
                    start_pos: Vec2::ZERO,
                    latest_pos: Vec2::ZERO,
                },
            );
        app.update();

        start_drag(&mut app, tab, window, PointerId::Mouse);
        drag_to(&mut app, tab, window, PointerId::Mouse, 20.0);
        assert_eq!(dragging(&app, tab), Some(PointerId::Mouse));

        app.world_mut()
            .resource_mut::<PointerState>()
            .clear(PointerId::Mouse);
        app.update();

        assert_eq!(dragging(&app, tab), None);
    }

    #[test]
    fn click_released_on_a_dragged_tab_does_not_request_selection() {
        let (mut app, window) = tab_app();
        let list = reorder_list(&mut app, window);
        let tab = placed_tab(&mut app, list, 50.0);
        app.update();

        start_drag(&mut app, tab, window, PointerId::Mouse);
        drag_to(&mut app, tab, window, PointerId::Mouse, 20.0);
        trigger_click(&mut app, tab, window, PointerButton::Primary);
        end_drag(&mut app, tab, window, 20.0);
        assert!(app.world().resource::<SelectionRequests>().0.is_empty());

        click(&mut app, tab, window);
        assert_eq!(
            app.world().resource::<SelectionRequests>().0,
            [(list, Some(tab))]
        );
    }

    #[test]
    fn click_after_a_drag_below_the_threshold_requests_selection() {
        let (mut app, window) = tab_app();
        let list = reorder_list(&mut app, window);
        let tab = placed_tab(&mut app, list, 50.0);
        app.update();

        start_drag(&mut app, tab, window, PointerId::Mouse);
        drag_to(&mut app, tab, window, PointerId::Mouse, 2.0);
        trigger_click(&mut app, tab, window, PointerButton::Primary);
        end_drag(&mut app, tab, window, 2.0);

        assert_eq!(
            app.world().resource::<SelectionRequests>().0,
            [(list, Some(tab))]
        );
        assert!(app.world().resource::<TabMoveLog>().0.is_empty());
    }

    #[test]
    fn external_drag_previews_on_the_hovered_list_and_proposes_a_cross_list_move() {
        let (mut app, window) = tab_app();
        let source = external_list(&mut app, window);
        let destination = external_list(&mut app, window);
        let dragged = placed_tab(&mut app, source, 50.0);
        placed_tab(&mut app, source, 150.0);
        placed_tab(&mut app, destination, 50.0);
        let target = placed_tab(&mut app, destination, 150.0);
        app.update();
        hover_only(&mut app, source, window);

        start_drag(&mut app, dragged, window, PointerId::Mouse);
        drag_to(&mut app, dragged, window, PointerId::Mouse, 200.0);
        assert_eq!(preview(&app, source).unwrap()[0].index, 1);
        assert_eq!(preview(&app, destination), None);

        hover_only(&mut app, target, window);
        drag_to(&mut app, dragged, window, PointerId::Mouse, 100.0);
        assert_eq!(preview(&app, source), None);
        assert_eq!(
            preview(&app, destination),
            Some(vec![TabInsertionPoint {
                pointer_id: PointerId::Mouse,
                tab: dragged,
                index: 1,
                slot: 1,
            }])
        );

        end_drag(&mut app, dragged, window, 100.0);
        assert_eq!(
            app.world().resource::<TabMoveLog>().0,
            [TabMoved {
                from_strip: source,
                tab: dragged,
                to_strip: destination,
                index: 1,
            }]
        );
        assert_eq!(
            app.world().resource::<DragLifecycleLog>().0,
            [
                TabDragLifecycle {
                    source,
                    tab: dragged,
                    pointer_id: PointerId::Mouse,
                    phase: TabDragPhase::Started,
                },
                TabDragLifecycle {
                    source,
                    tab: dragged,
                    pointer_id: PointerId::Mouse,
                    phase: TabDragPhase::Completed {
                        drop: Some(TabDrop {
                            destination,
                            index: 1,
                        }),
                    },
                },
            ]
        );
        assert_eq!(preview(&app, destination), None);
        assert_eq!(
            app.world().entity(dragged).get::<ChildOf>(),
            Some(&ChildOf(source))
        );
    }

    #[test]
    fn nearest_compatible_list_under_the_pointer_wins() {
        let (mut app, window) = tab_app();
        let source = external_list(&mut app, window);
        let near = external_list(&mut app, window);
        let far = external_list(&mut app, window);
        let covering = reorder_list(&mut app, window);
        let dragged = placed_tab(&mut app, source, 50.0);
        app.update();
        hover_at_depth(&mut app, PointerId::Mouse, far, window, 2.0);
        hover_at_depth(&mut app, PointerId::Mouse, near, window, 1.0);
        hover_at_depth(&mut app, PointerId::Mouse, covering, window, 0.0);

        start_drag(&mut app, dragged, window, PointerId::Mouse);
        drag_to(&mut app, dragged, window, PointerId::Mouse, 20.0);

        assert!(preview(&app, near).is_some());
        assert_eq!(preview(&app, far), None);
        assert_eq!(preview(&app, covering), None);
    }

    #[test]
    fn tabs_move_only_between_enabled_external_lists() {
        let (mut app, window) = tab_app();
        let reorder = reorder_list(&mut app, window);
        let external = external_list(&mut app, window);
        let disabled = external_list(&mut app, window);
        app.world_mut()
            .entity_mut(disabled)
            .insert(InteractionDisabled);
        let reorder_tab = placed_tab(&mut app, reorder, 50.0);
        let external_tab = placed_tab(&mut app, external, 50.0);
        app.update();

        for destination in [reorder, disabled] {
            hover_only(&mut app, destination, window);
            start_drag(&mut app, external_tab, window, PointerId::Mouse);
            drag_to(&mut app, external_tab, window, PointerId::Mouse, 20.0);
            assert_eq!(preview(&app, destination), None);
            end_drag(&mut app, external_tab, window, 20.0);
        }
        assert!(app.world().resource::<TabMoveLog>().0.is_empty());
        assert_eq!(
            phases(&app),
            [
                TabDragPhase::Started,
                TabDragPhase::Completed { drop: None },
            ]
            .repeat(2)
        );

        hover_only(&mut app, external, window);
        start_drag(&mut app, reorder_tab, window, PointerId::Mouse);
        drag_to(&mut app, reorder_tab, window, PointerId::Mouse, 20.0);
        assert_eq!(preview(&app, external), None);
        assert!(preview(&app, reorder).is_some());
        end_drag(&mut app, reorder_tab, window, 20.0);
        assert_eq!(
            app.world().resource::<TabMoveLog>().0,
            [TabMoved {
                from_strip: reorder,
                tab: reorder_tab,
                to_strip: reorder,
                index: 0,
            }]
        );
    }

    #[test]
    fn external_drag_over_no_compatible_list_has_no_preview() {
        let (mut app, window) = tab_app();
        let source = external_list(&mut app, window);
        let dragged = placed_tab(&mut app, source, 50.0);
        placed_tab(&mut app, source, 150.0);
        app.update();
        hover_only(&mut app, source, window);

        start_drag(&mut app, dragged, window, PointerId::Mouse);
        drag_to(&mut app, dragged, window, PointerId::Mouse, 200.0);
        assert_eq!(proposed(&app, source), Some((1, 2)));

        app.world_mut().resource_mut::<HoverMap>().clear();
        drag_to(&mut app, dragged, window, PointerId::Mouse, 200.0);
        assert_eq!(preview(&app, source), None);

        end_drag(&mut app, dragged, window, 200.0);
        assert!(app.world().resource::<TabMoveLog>().0.is_empty());
        assert_eq!(
            phases(&app),
            [
                TabDragPhase::Started,
                TabDragPhase::Completed { drop: None },
            ]
        );
    }

    #[test]
    fn lifecycle_reports_cancellation_once() {
        let (mut app, window) = tab_app();
        let list = external_list(&mut app, window);
        let first = placed_tab(&mut app, list, 50.0);
        let second = placed_tab(&mut app, list, 150.0);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(first, FocusCause::Navigated);
        app.update();
        hover(&mut app, PointerId::Mouse, list, window);

        start_drag(&mut app, first, window, PointerId::Mouse);
        drag_to(&mut app, first, window, PointerId::Mouse, 3.0);
        assert!(phases(&app).is_empty());

        drag_to(&mut app, first, window, PointerId::Mouse, 20.0);
        press_key(&mut app, KeyCode::Escape, window);
        end_drag(&mut app, first, window, 20.0);
        assert_eq!(
            phases(&app),
            [TabDragPhase::Started, TabDragPhase::Cancelled]
        );

        start_drag(&mut app, second, window, PointerId::Mouse);
        drag_to(&mut app, second, window, PointerId::Mouse, 20.0);
        app.world_mut().trigger(PointerCancel {
            entity: list,
            pointer: Pointer::new(PointerId::Mouse, window_location(window, Vec2::ZERO)),
            hit: HitData::new(window, 0.0, None, None),
        });
        app.update();

        start_drag(&mut app, first, window, PointerId::Mouse);
        drag_to(&mut app, first, window, PointerId::Mouse, 20.0);
        app.world_mut().despawn(first);
        app.update();

        assert_eq!(
            phases(&app),
            [TabDragPhase::Started, TabDragPhase::Cancelled].repeat(3)
        );
    }

    #[test]
    fn despawning_the_destination_mid_drag_completes_without_a_drop() {
        let (mut app, window) = tab_app();
        let source = external_list(&mut app, window);
        let destination = external_list(&mut app, window);
        let dragged = placed_tab(&mut app, source, 50.0);
        let target = placed_tab(&mut app, destination, 50.0);
        app.update();
        hover_only(&mut app, target, window);

        start_drag(&mut app, dragged, window, PointerId::Mouse);
        drag_to(&mut app, dragged, window, PointerId::Mouse, 20.0);
        assert!(preview(&app, destination).is_some());

        app.world_mut().entity_mut(destination).despawn();
        app.update();
        assert_eq!(dragging(&app, dragged), Some(PointerId::Mouse));
        assert_eq!(preview(&app, source), None);

        end_drag(&mut app, dragged, window, 20.0);
        assert!(app.world().resource::<TabMoveLog>().0.is_empty());
        assert_eq!(
            phases(&app),
            [
                TabDragPhase::Started,
                TabDragPhase::Completed { drop: None },
            ]
        );
        assert_eq!(dragging(&app, dragged), None);
    }

    #[test]
    fn the_last_tab_can_leave_its_list_and_return_to_it_empty() {
        let (mut app, window) = tab_app();
        app.add_observer(|moved: On<TabMoved>, mut commands: Commands| {
            commands
                .entity(moved.to_strip)
                .insert_child(moved.index, moved.tab);
        });
        let source = external_list(&mut app, window);
        let destination = external_list(&mut app, window);
        let dragged = placed_tab(&mut app, source, 50.0);
        let resident = placed_tab(&mut app, destination, 50.0);
        app.world_mut()
            .entity_mut(source)
            .insert(SelectedTab(Some(dragged)));
        app.update();

        hover_only(&mut app, resident, window);
        start_drag(&mut app, dragged, window, PointerId::Mouse);
        drag_to(&mut app, dragged, window, PointerId::Mouse, 20.0);
        end_drag(&mut app, dragged, window, 20.0);
        assert!(app.world().entity(source).get::<Children>().is_none());
        assert_eq!(
            app.world()
                .entity(destination)
                .get::<Children>()
                .unwrap()
                .as_ref(),
            [dragged, resident]
        );
        assert!(!app.world().entity(dragged).contains::<Selected>());

        hover_only(&mut app, source, window);
        start_drag(&mut app, dragged, window, PointerId::Mouse);
        drag_to(&mut app, dragged, window, PointerId::Mouse, 20.0);
        assert_eq!(preview(&app, source).unwrap()[0].index, 0);
        end_drag(&mut app, dragged, window, 20.0);
        assert_eq!(
            app.world()
                .entity(source)
                .get::<Children>()
                .unwrap()
                .as_ref(),
            [dragged]
        );
    }
}
