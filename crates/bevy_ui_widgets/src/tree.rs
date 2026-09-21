use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::{Entity, EntityHashSet},
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Changed, Has, Or, With},
    reflect::{ReflectComponent, ReflectEvent},
    schedule::IntoScheduleConfigs,
    system::{Commands, ParamSet, Query, Res, ResMut},
    template::FromTemplate,
    world::World,
};
use bevy_input::{
    keyboard::{KeyCode, KeyboardInput},
    ButtonState,
};
use bevy_input_focus::{
    tab_navigation::TabIndex, FocusCause, FocusedInput, InputFocus, InputFocusSystems,
    InputFocusVisible,
};
use bevy_picking::{events::PointerClick, pointer::PointerButton};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{Expandable, Expanded, InteractionDisabled, Selectable, Selected};

/// Determines how many rows of a [`TreeView`] may be selected at once.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default, Clone, PartialEq)]
pub enum TreeSelectionMode {
    /// Rows are never selected; the tree only emits activation and expansion events.
    None,
    /// At most one row is selected.
    #[default]
    Single,
}

/// Headless tree-view behavior and policy.
///
/// A tree nests as `TreeView -> TreeItem -> TreeItemChildren -> TreeItem`. Rows are never spawned
/// by the widget. Selection lives in [`SelectedTreeItem`] and expansion in the [`Expanded`]
/// marker; both only change when [`tree_view_self_update`] and [`tree_view_expand_self_update`]
/// are added.
#[derive(Component, Debug, Default, Clone, PartialEq, Reflect)]
#[require(AccessibilityNode(accesskit::Node::new(Role::Tree)), SelectedTreeItem)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct TreeView {
    /// How many rows may be selected at once.
    pub selection: TreeSelectionMode,
    /// Top-to-bottom order of this tree's visible rows, refreshed in `PostUpdate` when the tree changes.
    pub visible_rows: Vec<Entity>,
}

/// The selected [`TreeItem`] within a [`TreeView`]. The referenced entity must be an enabled row
/// of that tree; missing, stale, disabled, and unrelated entities count as no selection.
#[derive(Component, FromTemplate, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, PartialEq)]
pub struct SelectedTreeItem(#[template(built_in)] pub Option<Entity>);

/// A headless tree row. Rows are focusable through a roving [`TabIndex`], and derive [`Selected`]
/// from the containing tree's valid [`SelectedTreeItem`] value and [`Expandable`] from
/// `has_children`. A row is expanded while it holds the [`Expanded`] marker.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[require(
    AccessibilityNode(accesskit::Node::new(Role::TreeItem)),
    Selectable,
    TabIndex(-1)
)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct TreeItem {
    /// Whether this row can be expanded. Rows without children never expand.
    pub has_children: bool,
    /// Nesting depth of this row, where top-level rows are zero. Derived in `PostUpdate`.
    pub level: u32,
}

/// Container for a [`TreeItem`]'s child rows. It must be a direct child of the row it belongs to,
/// and child rows must be its direct children.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct TreeItemChildren;

/// Marker for a disclosure control inside a [`TreeItem`]. Clicking it requests expansion or
/// collapse of the enclosing row instead of selecting it.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct TreeItemToggle;

/// Notification sent by a [`TreeView`] when the user commits a row by clicking it or by pressing
/// Enter or Space. Sent even when the row was already selected.
#[derive(Copy, Clone, Debug, PartialEq, EntityEvent, Reflect)]
#[reflect(Event)]
pub struct TreeItemActivate {
    /// The tree that produced this event.
    #[event_target]
    pub tree: Entity,
    /// The activated row.
    pub item: Entity,
}

/// Notification sent by a [`TreeView`] when the user requests that a row be expanded or collapsed.
#[derive(Copy, Clone, Debug, PartialEq, EntityEvent, Reflect)]
#[reflect(Event)]
pub struct TreeItemExpandChange {
    /// The tree that produced this event.
    #[event_target]
    pub tree: Entity,
    /// The row whose expansion was requested.
    pub item: Entity,
    /// The requested expansion state.
    pub expanded: bool,
}

/// Plugin that registers tree-view observers and derived state.
pub struct TreePlugin;

impl Plugin for TreePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(tree_view_on_click)
            .add_observer(tree_item_on_key_input)
            .add_systems(
                PostUpdate,
                (
                    update_tree_item_levels,
                    update_visible_rows,
                    update_tree_view_derived_state,
                )
                    .chain()
                    .after(crate::MenuFocusSystem)
                    .before(InputFocusSystems::FocusChangeEvents),
            );
    }
}

/// Observer that applies row selection requests to [`SelectedTreeItem`].
pub fn tree_view_self_update(
    change: On<crate::ValueChange<Option<Entity>>>,
    trees: Query<(), With<TreeView>>,
    mut commands: Commands,
) {
    if trees.contains(change.source) {
        commands
            .entity(change.source)
            .insert(SelectedTreeItem(change.value));
    }
}

/// Observer that applies expansion requests by adding or removing [`Expanded`].
pub fn tree_view_expand_self_update(
    change: On<TreeItemExpandChange>,
    items: Query<Has<Expanded>, With<TreeItem>>,
    mut commands: Commands,
) {
    let Ok(expanded) = items.get(change.item) else {
        return;
    };
    if expanded == change.expanded {
        return;
    }
    if change.expanded {
        commands.entity(change.item).insert(Expanded);
    } else {
        commands.entity(change.item).remove::<Expanded>();
    }
}

type RowQuery<'w, 's> = Query<'w, 's, (&'static TreeItem, Has<Expanded>, Has<InteractionDisabled>)>;
type ContainerQuery<'w, 's> = Query<'w, 's, (), With<TreeItemChildren>>;

fn visible_rows(
    container: Entity,
    children: &Query<&Children>,
    rows: &RowQuery,
    containers: &ContainerQuery,
    out: &mut Vec<Entity>,
) {
    let Ok(container_children) = children.get(container) else {
        return;
    };
    for child in container_children.iter().copied() {
        let Ok((_, expanded, disabled)) = rows.get(child) else {
            continue;
        };
        if disabled {
            continue;
        }
        out.push(child);
        if expanded {
            for row_child in child_containers(child, children, containers) {
                visible_rows(row_child, children, rows, containers, out);
            }
        }
    }
}

fn child_containers<'a>(
    row: Entity,
    children: &'a Query<&Children>,
    containers: &'a ContainerQuery,
) -> impl Iterator<Item = Entity> + 'a {
    children
        .get(row)
        .into_iter()
        .flat_map(|row_children| row_children.iter().copied())
        .filter(move |child| containers.contains(*child))
}

fn first_child_row(
    row: Entity,
    children: &Query<&Children>,
    rows: &RowQuery,
    containers: &ContainerQuery,
) -> Option<Entity> {
    child_containers(row, children, containers)
        .filter_map(|container| {
            children.get(container).ok().and_then(|nested| {
                nested
                    .iter()
                    .copied()
                    .find(|child| rows.get(*child).is_ok_and(|(.., disabled)| !disabled))
            })
        })
        .next()
}

fn parent_row(
    row: Entity,
    parents: &Query<&ChildOf>,
    rows: &RowQuery,
    containers: &ContainerQuery,
) -> Option<Entity> {
    let container = parents.get(row).ok()?.parent();
    if !containers.contains(container) {
        return None;
    }
    let parent = parents.get(container).ok()?.parent();
    rows.get(parent)
        .ok()
        .and_then(|(.., disabled)| if disabled { None } else { Some(parent) })
}

fn owning_tree(
    row: Entity,
    parents: &Query<&ChildOf>,
    trees: &Query<(&TreeView, &SelectedTreeItem, Has<InteractionDisabled>)>,
) -> Option<Entity> {
    parents
        .iter_ancestors(row)
        .find(|ancestor| trees.contains(*ancestor))
}

/// Returns the set of [`TreeView`] entities where at least one child has changed.
fn owning_trees(
    changed_entities: impl Iterator<Item = Entity>,
    parents: &Query<&ChildOf>,
    trees: &Query<Entity, With<TreeView>>,
) -> EntityHashSet {
    changed_entities
        .filter_map(|entity| {
            if trees.contains(entity) {
                Some(entity)
            } else {
                parents
                    .iter_ancestors(entity)
                    .find(|ancestor| trees.contains(*ancestor))
            }
        })
        .collect()
}

fn request_selection(
    view: &TreeView,
    selection: &SelectedTreeItem,
    tree: Entity,
    item: Entity,
    commands: &mut Commands,
) {
    if view.selection == TreeSelectionMode::Single && selection.0 != Some(item) {
        commands.trigger(crate::ValueChange::<Option<Entity>> {
            source: tree,
            value: Some(item),
            is_final: true,
        });
    }
}

fn tree_view_on_click(
    mut click: On<PointerClick>,
    trees: Query<(&TreeView, &SelectedTreeItem, Has<InteractionDisabled>)>,
    rows: RowQuery,
    toggles: Query<(), With<TreeItemToggle>>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    let Ok((view, selection, tree_disabled)) = trees.get(click.entity) else {
        return;
    };

    let target = click.original_event_target();
    let mut toggle = toggles.contains(target);
    let row = if rows.contains(target) {
        Some(target)
    } else {
        parents
            .iter_ancestors(target)
            .take_while(|ancestor| *ancestor != click.entity)
            .inspect(|ancestor| toggle |= toggles.contains(*ancestor))
            .find(|ancestor| rows.contains(*ancestor))
    };
    let Some(row) = row else {
        return;
    };
    if owning_tree(row, &parents, &trees) != Some(click.entity) {
        return;
    }
    let Ok((item, expanded, row_disabled)) = rows.get(row) else {
        return;
    };
    if tree_disabled || row_disabled {
        return;
    }

    click.propagate(false);
    if toggle {
        if item.has_children {
            commands.trigger(TreeItemExpandChange {
                tree: click.entity,
                item: row,
                expanded: !expanded,
            });
        }
        return;
    }
    request_selection(view, selection, click.entity, row, &mut commands);
    commands.trigger(TreeItemActivate {
        tree: click.entity,
        item: row,
    });
}

fn tree_item_on_key_input(
    mut input: On<FocusedInput<KeyboardInput>>,
    trees: Query<(&TreeView, &SelectedTreeItem, Has<InteractionDisabled>)>,
    rows: RowQuery,
    containers: ContainerQuery,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    mut focus: ResMut<InputFocus>,
    mut focus_visible: ResMut<InputFocusVisible>,
    mut commands: Commands,
) {
    let row = input.focused_entity;
    let Ok((item, expanded, row_disabled)) = rows.get(row) else {
        return;
    };
    let Some(tree) = owning_tree(row, &parents, &trees) else {
        return;
    };
    let Ok((view, selection, tree_disabled)) = trees.get(tree) else {
        return;
    };
    if tree_disabled || row_disabled {
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
        In,
        Out,
        Activate,
    }

    let navigation = match event.key_code {
        KeyCode::ArrowUp => Navigation::Previous,
        KeyCode::ArrowDown => Navigation::Next,
        KeyCode::Home => Navigation::First,
        KeyCode::End => Navigation::Last,
        KeyCode::ArrowRight => Navigation::In,
        KeyCode::ArrowLeft => Navigation::Out,
        KeyCode::Enter | KeyCode::Space => Navigation::Activate,
        _ => return,
    };
    input.propagate(false);

    match navigation {
        Navigation::Activate => {
            request_selection(view, selection, tree, row, &mut commands);
            commands.trigger(TreeItemActivate { tree, item: row });
            return;
        }
        Navigation::In if item.has_children && !expanded => {
            commands.trigger(TreeItemExpandChange {
                tree,
                item: row,
                expanded: true,
            });
            return;
        }
        Navigation::Out if item.has_children && expanded => {
            commands.trigger(TreeItemExpandChange {
                tree,
                item: row,
                expanded: false,
            });
            return;
        }
        _ => {}
    }

    let next = match navigation {
        Navigation::In => first_child_row(row, &children, &rows, &containers),
        Navigation::Out => parent_row(row, &parents, &rows, &containers),
        _ => {
            let current = view.visible_rows.iter().position(|visible| *visible == row);
            match navigation {
                Navigation::Previous => current
                    .filter(|index| *index > 0)
                    .map(|index| view.visible_rows[index - 1]),
                Navigation::Next => current
                    .filter(|index| *index + 1 < view.visible_rows.len())
                    .map(|index| view.visible_rows[index + 1]),
                Navigation::First => view.visible_rows.first().copied(),
                Navigation::Last => view.visible_rows.last().copied(),
                _ => None,
            }
        }
    };

    let Some(next) = next else {
        return;
    };
    if focus.get() != Some(next) {
        focus.set(next, FocusCause::Navigated);
    }
    focus_visible.0 = true;
    request_selection(view, selection, tree, next, &mut commands);
}

/// Derives [`TreeItem::level`] from the nesting of rows and [`TreeItemChildren`] containers.
///
/// Runs in `PostUpdate`, early-returning unless the tree hierarchy changed.
fn update_tree_item_levels(
    trees: Query<Entity, With<TreeView>>,
    children: Query<&Children>,
    containers: Query<(), With<TreeItemChildren>>,
    parents: Query<&ChildOf>,
    mut queries: ParamSet<(
        Query<
            Entity,
            (
                Or<(Changed<Children>, Added<TreeItem>)>,
                Or<(With<TreeView>, With<TreeItem>, With<TreeItemChildren>)>,
            ),
        >,
        Query<&mut TreeItem>,
    )>,
) {
    let changed_trees = owning_trees(queries.p0().iter(), &parents, &trees);
    if changed_trees.is_empty() {
        return;
    }

    let mut levels = Vec::new();
    let mut items = queries.p1();
    for tree in changed_trees {
        collect_levels(tree, 0, &children, &containers, &items, &mut levels);
    }
    for (row, level) in levels {
        if let Ok(mut item) = items.get_mut(row)
            && item.level != level
        {
            item.level = level;
        }
    }
}

fn collect_levels(
    container: Entity,
    level: u32,
    children: &Query<&Children>,
    containers: &Query<(), With<TreeItemChildren>>,
    items: &Query<&mut TreeItem>,
    out: &mut Vec<(Entity, u32)>,
) {
    let Ok(container_children) = children.get(container) else {
        return;
    };
    for child in container_children.iter().copied() {
        if !items.contains(child) {
            continue;
        }
        out.push((child, level));
        let Ok(row_children) = children.get(child) else {
            continue;
        };
        for nested in row_children.iter().copied() {
            if containers.contains(nested) {
                collect_levels(nested, level + 1, children, containers, items, out);
            }
        }
    }
}

fn update_visible_rows(
    trees: Query<Entity, With<TreeView>>,
    children: Query<&Children>,
    rows: RowQuery,
    containers: ContainerQuery,
    parents: Query<&ChildOf>,
    mut views: Query<&mut TreeView>,
    changed: Query<
        Entity,
        (
            Or<(
                Changed<Children>,
                Changed<TreeItem>,
                Added<Expanded>,
                Added<InteractionDisabled>,
            )>,
            Or<(With<TreeView>, With<TreeItem>, With<TreeItemChildren>)>,
        ),
    >,
    mut removed_expanded: RemovedComponents<Expanded>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
) {
    let changed_trees = owning_trees(
        changed
            .iter()
            .chain(removed_expanded.read())
            .chain(removed_disabled.read()),
        &parents,
        &trees,
    );

    for tree in changed_trees {
        let mut visible_rows_buffer = Vec::new();
        visible_rows(
            tree,
            &children,
            &rows,
            &containers,
            &mut visible_rows_buffer,
        );
        if let Ok(mut view) = views.get_mut(tree) {
            view.visible_rows = visible_rows_buffer;
        }
    }
}

/// Derives [`Selected`], [`Expandable`] and the roving [`TabIndex`] from each tree's validated
/// [`SelectedTreeItem`], row `has_children` and the current focus, in `PostUpdate`, only when
/// relevant state changed.
fn update_tree_view_derived_state(
    trees: Query<(
        Entity,
        &TreeView,
        &SelectedTreeItem,
        Has<InteractionDisabled>,
    )>,
    children: Query<&Children>,
    rows: RowQuery,
    row_state: Query<(&TreeItem, Has<Selected>, Has<Expandable>, &TabIndex)>,
    focus: Option<Res<InputFocus>>,
    changed_trees: Query<
        (),
        (
            With<TreeView>,
            Or<(Changed<SelectedTreeItem>, Changed<Children>)>,
        ),
    >,
    changed_rows: Query<
        (),
        (
            With<TreeItem>,
            Or<(
                Changed<TreeItem>,
                Changed<Children>,
                Added<Expanded>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    changed_containers: Query<(), (With<TreeItemChildren>, Changed<Children>)>,
    mut removed_expanded: RemovedComponents<Expanded>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    let focus_changed = focus.as_ref().is_some_and(DetectChanges::is_changed);
    let expanded_removed = !removed_expanded.is_empty();
    let disabled_removed = !removed_disabled.is_empty();
    removed_expanded.clear();
    removed_disabled.clear();
    if !focus_changed
        && !expanded_removed
        && !disabled_removed
        && changed_trees.is_empty()
        && changed_rows.is_empty()
        && changed_containers.is_empty()
    {
        return;
    }

    for (tree, view, selection, tree_disabled) in trees.iter() {
        let tree_rows = children
            .iter_descendants(tree)
            .filter(|descendant| rows.contains(*descendant))
            .collect::<EntityHashSet>();
        let enabled = |entity: &Entity| {
            tree_rows.contains(entity) && rows.get(*entity).is_ok_and(|(.., disabled)| !disabled)
        };
        let visible_set = view.visible_rows.iter().copied().collect::<EntityHashSet>();

        let current_focus = focus.as_deref().and_then(InputFocus::get);
        let selected = selection.0.filter(enabled);
        let focused =
            current_focus.filter(|entity| enabled(entity) && visible_set.contains(entity));
        let roving = focused
            .or_else(|| selected.filter(|entity| visible_set.contains(entity)))
            .or_else(|| view.visible_rows.first().copied());

        if let Some(focused_entity) = current_focus
            && !tree_disabled
            && tree_rows.contains(&focused_entity)
            && Some(focused_entity) != roving
            && let Some(roving) = roving
        {
            commands.queue(move |world: &mut World| {
                world
                    .resource_mut::<InputFocus>()
                    .set(roving, FocusCause::Navigated);
            });
        }

        for row in tree_rows.iter().copied() {
            let Ok((item, is_selected, is_expandable, tab_index)) = row_state.get(row) else {
                continue;
            };
            if item.has_children && !is_expandable {
                commands.entity(row).insert(Expandable);
            } else if !item.has_children && is_expandable {
                commands
                    .entity(row)
                    .remove::<Expanded>()
                    .remove::<Expandable>();
            }

            let should_select = selected == Some(row);
            if should_select && !is_selected {
                commands.entity(row).insert(Selected);
            } else if !should_select && is_selected {
                commands.entity(row).remove::<Selected>();
            }

            let desired_index = if roving == Some(row) { 0 } else { -1 };
            if tab_index.0 != desired_index {
                commands.entity(row).insert(TabIndex(desired_index));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::{resource::Resource, system::ResMut};
    use bevy_input::{keyboard::Key, InputPlugin};
    use bevy_input_focus::{InputDispatchPlugin, InputFocusPlugin};
    use bevy_math::Vec2;
    use bevy_picking::{
        backend::HitData,
        events::Pointer,
        pointer::{Location, PointerId},
    };
    use bevy_window::{PrimaryWindow, Window, WindowRef};

    #[derive(Resource, Default)]
    struct SelectionRequests(Vec<(Entity, Option<Entity>)>);

    #[derive(Resource, Default)]
    struct ExpandRequests(Vec<(Entity, Entity, bool)>);

    #[derive(Resource, Default)]
    struct Activations(Vec<(Entity, Entity)>);

    fn tree_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((
            InputPlugin,
            InputFocusPlugin,
            InputDispatchPlugin,
            TreePlugin,
        ))
        .init_resource::<SelectionRequests>()
        .init_resource::<ExpandRequests>()
        .init_resource::<Activations>()
        .add_observer(
            |change: On<crate::ValueChange<Option<Entity>>>,
             mut requests: ResMut<SelectionRequests>| {
                requests.0.push((change.source, change.value));
            },
        )
        .add_observer(
            |change: On<TreeItemExpandChange>, mut requests: ResMut<ExpandRequests>| {
                requests.0.push((change.tree, change.item, change.expanded));
            },
        )
        .add_observer(
            |activate: On<TreeItemActivate>, mut activations: ResMut<Activations>| {
                activations.0.push((activate.tree, activate.item));
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
            KeyCode::Enter => Key::Enter,
            KeyCode::Space => Key::Space,
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

    fn click_with_button(app: &mut App, target: Entity, window: Entity, button: PointerButton) {
        let location = Location {
            target: bevy_camera::NormalizedRenderTarget::Window(
                WindowRef::Entity(window).normalize(Some(window)).unwrap(),
            ),
            position: Vec2::ZERO,
        };
        app.world_mut().trigger(PointerClick {
            entity: target,
            pointer: Pointer::new(PointerId::Mouse, location),
            button,
            hit: HitData::new(window, 0.0, None, None),
            duration: core::time::Duration::from_millis(10),
            count: 1,
        });
        app.update();
    }

    struct Fixture {
        tree: Entity,
        first: Entity,
        parent: Entity,
        child_a: Entity,
        child_b: Entity,
        last: Entity,
    }

    /// Builds `first`, `parent` (expanded, with `child_a` and `child_b`), and `last`.
    fn spawn_tree(app: &mut App, window: Entity, expanded: bool) -> Fixture {
        let tree = app
            .world_mut()
            .spawn((TreeView::default(), ChildOf(window)))
            .id();
        let first = app
            .world_mut()
            .spawn((TreeItem::default(), ChildOf(tree)))
            .id();
        let mut parent_entity = app.world_mut().spawn((
            TreeItem {
                has_children: true,
                level: 0,
            },
            ChildOf(tree),
        ));
        if expanded {
            parent_entity.insert(Expanded);
        }
        let parent = parent_entity.id();
        let container = app
            .world_mut()
            .spawn((TreeItemChildren, ChildOf(parent)))
            .id();
        let child_a = app
            .world_mut()
            .spawn((TreeItem::default(), ChildOf(container)))
            .id();
        let child_b = app
            .world_mut()
            .spawn((TreeItem::default(), ChildOf(container)))
            .id();
        let last = app
            .world_mut()
            .spawn((TreeItem::default(), ChildOf(tree)))
            .id();
        app.update();
        Fixture {
            tree,
            first,
            parent,
            child_a,
            child_b,
            last,
        }
    }

    fn focus(app: &mut App, row: Entity) {
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(row, FocusCause::Navigated);
        app.update();
    }

    #[test]
    fn tree_view_and_item_install_accessibility_semantics() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);

        let tree_node = app
            .world()
            .entity(fixture.tree)
            .get::<AccessibilityNode>()
            .unwrap();
        let row_node = app
            .world()
            .entity(fixture.first)
            .get::<AccessibilityNode>()
            .unwrap();
        assert_eq!(tree_node.role(), Role::Tree);
        assert_eq!(row_node.role(), Role::TreeItem);
        assert!(app.world().entity(fixture.first).contains::<Selectable>());
    }

    #[test]
    fn clicking_row_requests_selection_without_mutating_controlled_state() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        app.world_mut()
            .entity_mut(fixture.tree)
            .insert(SelectedTreeItem(Some(fixture.first)));
        app.update();

        click(&mut app, fixture.last, window);

        assert_eq!(
            app.world().resource::<SelectionRequests>().0,
            [(fixture.tree, Some(fixture.last))]
        );
        assert_eq!(
            app.world().resource::<Activations>().0,
            [(fixture.tree, fixture.last)]
        );
        assert_eq!(
            app.world().entity(fixture.tree).get::<SelectedTreeItem>(),
            Some(&SelectedTreeItem(Some(fixture.first)))
        );
    }

    #[test]
    fn self_update_observer_applies_selection_request() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        app.world_mut()
            .entity_mut(fixture.tree)
            .observe(tree_view_self_update);
        app.update();

        click(&mut app, fixture.child_b, window);

        assert_eq!(
            app.world().entity(fixture.tree).get::<SelectedTreeItem>(),
            Some(&SelectedTreeItem(Some(fixture.child_b)))
        );
    }

    #[test]
    fn expand_self_update_observer_applies_expansion_request() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, false);
        app.world_mut()
            .entity_mut(fixture.tree)
            .observe(tree_view_expand_self_update);
        focus(&mut app, fixture.parent);

        press_key(&mut app, KeyCode::ArrowRight, window);

        assert!(app.world().entity(fixture.parent).contains::<Expanded>());
    }

    #[test]
    fn clicking_toggle_requests_expansion_instead_of_selection() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, false);
        let toggle = app
            .world_mut()
            .spawn((TreeItemToggle, ChildOf(fixture.parent)))
            .id();
        app.update();

        click(&mut app, toggle, window);

        assert_eq!(
            app.world().resource::<ExpandRequests>().0,
            [(fixture.tree, fixture.parent, true)]
        );
        assert!(app.world().resource::<SelectionRequests>().0.is_empty());
    }

    #[test]
    fn arrow_keys_move_through_visible_rows_only() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, false);
        focus(&mut app, fixture.first);

        press_key(&mut app, KeyCode::ArrowDown, window);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.parent)
        );

        press_key(&mut app, KeyCode::ArrowDown, window);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.last)
        );

        press_key(&mut app, KeyCode::ArrowUp, window);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.parent)
        );
    }

    #[test]
    fn arrow_keys_enter_expanded_subtrees() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        focus(&mut app, fixture.parent);

        press_key(&mut app, KeyCode::ArrowDown, window);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.child_a)
        );

        press_key(&mut app, KeyCode::ArrowDown, window);
        press_key(&mut app, KeyCode::ArrowDown, window);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.last)
        );
    }

    #[test]
    fn navigation_requests_selection_for_the_newly_focused_row() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, false);
        focus(&mut app, fixture.first);

        press_key(&mut app, KeyCode::ArrowDown, window);

        assert_eq!(
            app.world().resource::<SelectionRequests>().0,
            [(fixture.tree, Some(fixture.parent))]
        );
    }

    #[test]
    fn right_arrow_expands_then_moves_to_first_child() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, false);
        focus(&mut app, fixture.parent);

        press_key(&mut app, KeyCode::ArrowRight, window);
        assert_eq!(
            app.world().resource::<ExpandRequests>().0,
            [(fixture.tree, fixture.parent, true)]
        );
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.parent)
        );

        app.world_mut().entity_mut(fixture.parent).insert(Expanded);
        app.update();

        press_key(&mut app, KeyCode::ArrowRight, window);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.child_a)
        );
    }

    #[test]
    fn left_arrow_collapses_then_moves_to_parent() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        focus(&mut app, fixture.parent);

        press_key(&mut app, KeyCode::ArrowLeft, window);
        assert_eq!(
            app.world().resource::<ExpandRequests>().0,
            [(fixture.tree, fixture.parent, false)]
        );

        focus(&mut app, fixture.child_b);
        press_key(&mut app, KeyCode::ArrowLeft, window);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.parent)
        );
    }

    #[test]
    fn home_and_end_move_to_first_and_last_visible_rows() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        focus(&mut app, fixture.parent);

        press_key(&mut app, KeyCode::End, window);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.last)
        );

        press_key(&mut app, KeyCode::Home, window);
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.first)
        );
    }

    #[test]
    fn rows_request_selection_on_enter_and_space() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        focus(&mut app, fixture.first);

        press_key(&mut app, KeyCode::Enter, window);
        press_key(&mut app, KeyCode::Space, window);

        assert_eq!(
            app.world().resource::<SelectionRequests>().0,
            [
                (fixture.tree, Some(fixture.first)),
                (fixture.tree, Some(fixture.first))
            ]
        );
        assert_eq!(
            app.world().resource::<Activations>().0,
            [(fixture.tree, fixture.first), (fixture.tree, fixture.first)]
        );
    }

    #[test]
    fn disabled_rows_are_skipped_by_navigation() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        app.world_mut()
            .entity_mut(fixture.child_a)
            .insert(InteractionDisabled);
        focus(&mut app, fixture.parent);

        press_key(&mut app, KeyCode::ArrowDown, window);

        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.child_b)
        );
    }

    #[test]
    fn disabled_row_click_does_not_request_selection() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        app.world_mut()
            .entity_mut(fixture.first)
            .insert(InteractionDisabled);
        app.update();

        click(&mut app, fixture.first, window);

        assert!(app.world().resource::<SelectionRequests>().0.is_empty());
    }

    #[test]
    fn disabled_tree_ignores_clicks_and_keys() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        app.world_mut()
            .entity_mut(fixture.tree)
            .insert(InteractionDisabled);
        focus(&mut app, fixture.first);

        click(&mut app, fixture.last, window);
        press_key(&mut app, KeyCode::ArrowDown, window);
        press_key(&mut app, KeyCode::Enter, window);

        assert!(app.world().resource::<SelectionRequests>().0.is_empty());
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.first)
        );
    }

    #[test]
    fn secondary_click_does_not_change_selection() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);

        click_with_button(&mut app, fixture.first, window, PointerButton::Secondary);
        click_with_button(&mut app, fixture.first, window, PointerButton::Middle);

        assert!(app.world().resource::<SelectionRequests>().0.is_empty());
    }

    #[test]
    fn valid_selection_derives_selected_state_and_roving_entry() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        app.world_mut()
            .entity_mut(fixture.tree)
            .insert(SelectedTreeItem(Some(fixture.child_b)));
        app.update();

        assert!(app.world().entity(fixture.child_b).contains::<Selected>());
        assert!(!app.world().entity(fixture.first).contains::<Selected>());
        assert_eq!(
            app.world().entity(fixture.child_b).get::<TabIndex>(),
            Some(&TabIndex(0))
        );
        assert_eq!(
            app.world().entity(fixture.first).get::<TabIndex>(),
            Some(&TabIndex(-1))
        );

        app.world_mut()
            .entity_mut(fixture.tree)
            .insert(SelectedTreeItem(Some(fixture.first)));
        app.update();

        assert!(!app.world().entity(fixture.child_b).contains::<Selected>());
        assert!(app.world().entity(fixture.first).contains::<Selected>());
    }

    #[test]
    fn invalid_selection_clears_selected_state_and_uses_first_visible_roving_entry() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        let stale = app.world_mut().spawn_empty().id();
        app.world_mut()
            .entity_mut(fixture.tree)
            .insert(SelectedTreeItem(Some(stale)));
        app.update();

        assert!(!app.world().entity(fixture.first).contains::<Selected>());
        assert_eq!(
            app.world().entity(fixture.first).get::<TabIndex>(),
            Some(&TabIndex(0))
        );
    }

    #[test]
    fn levels_are_derived_from_nesting() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);

        assert_eq!(
            app.world()
                .entity(fixture.parent)
                .get::<TreeItem>()
                .unwrap()
                .level,
            0
        );
        assert_eq!(
            app.world()
                .entity(fixture.child_a)
                .get::<TreeItem>()
                .unwrap()
                .level,
            1
        );
    }

    #[test]
    fn hiding_the_focused_row_updates_tab_index_and_focus() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        focus(&mut app, fixture.child_a);
        assert_eq!(
            app.world().entity(fixture.child_a).get::<TabIndex>(),
            Some(&TabIndex(0))
        );

        app.world_mut()
            .entity_mut(fixture.parent)
            .remove::<Expanded>();
        app.update();

        assert_eq!(
            app.world().entity(fixture.child_a).get::<TabIndex>(),
            Some(&TabIndex(-1))
        );
        press_key(&mut app, KeyCode::ArrowDown, window);
        assert_ne!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.child_a)
        );
    }

    #[test]
    fn disabled_tree_does_not_steal_focus_when_a_row_becomes_hidden() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        app.world_mut()
            .entity_mut(fixture.tree)
            .insert(InteractionDisabled);
        focus(&mut app, fixture.child_a);

        app.world_mut()
            .entity_mut(fixture.parent)
            .remove::<Expanded>();
        app.update();

        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.child_a)
        );
    }

    #[test]
    fn disabled_parent_hides_its_children_from_navigation() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);
        app.world_mut()
            .entity_mut(fixture.parent)
            .insert(InteractionDisabled);
        focus(&mut app, fixture.first);

        press_key(&mut app, KeyCode::ArrowDown, window);

        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(fixture.last)
        );
    }

    #[test]
    fn inserting_tree_item_without_a_children_change_updates_level() {
        let (mut app, window) = tree_app();
        let tree = app
            .world_mut()
            .spawn((TreeView::default(), ChildOf(window)))
            .id();
        let parent = app
            .world_mut()
            .spawn((
                TreeItem {
                    has_children: true,
                    level: 0,
                },
                Expanded,
                ChildOf(tree),
            ))
            .id();
        let container = app
            .world_mut()
            .spawn((TreeItemChildren, ChildOf(parent)))
            .id();
        app.update();

        let late_row = app.world_mut().spawn(ChildOf(container)).id();
        app.update();

        app.world_mut()
            .entity_mut(late_row)
            .insert(TreeItem::default());
        app.update();

        assert_eq!(
            app.world()
                .entity(late_row)
                .get::<TreeItem>()
                .unwrap()
                .level,
            1
        );
    }

    #[test]
    fn reparenting_a_row_into_a_collapsed_branch_updates_its_tab_index() {
        let (mut app, window) = tree_app();
        let tree = app
            .world_mut()
            .spawn((TreeView::default(), ChildOf(window)))
            .id();
        let parent_a = app
            .world_mut()
            .spawn((
                TreeItem {
                    has_children: true,
                    level: 0,
                },
                Expanded,
                ChildOf(tree),
            ))
            .id();
        let container_a = app
            .world_mut()
            .spawn((TreeItemChildren, ChildOf(parent_a)))
            .id();
        let moved_row = app
            .world_mut()
            .spawn((
                TreeItem {
                    level: 1,
                    ..Default::default()
                },
                ChildOf(container_a),
            ))
            .id();
        let parent_b = app
            .world_mut()
            .spawn((
                TreeItem {
                    has_children: true,
                    level: 0,
                },
                ChildOf(tree),
            ))
            .id();
        let container_b = app
            .world_mut()
            .spawn((TreeItemChildren, ChildOf(parent_b)))
            .id();
        app.update();
        focus(&mut app, moved_row);
        assert_eq!(
            app.world().entity(moved_row).get::<TabIndex>(),
            Some(&TabIndex(0))
        );

        app.world_mut()
            .entity_mut(moved_row)
            .insert(ChildOf(container_b));
        app.update();

        assert_eq!(
            app.world().entity(moved_row).get::<TabIndex>(),
            Some(&TabIndex(-1))
        );
    }

    #[test]
    fn expandable_is_derived_from_has_children() {
        let (mut app, window) = tree_app();
        let fixture = spawn_tree(&mut app, window, true);

        assert!(app.world().entity(fixture.parent).contains::<Expandable>());
        assert!(!app.world().entity(fixture.first).contains::<Expandable>());

        app.world_mut().entity_mut(fixture.parent).insert(TreeItem {
            has_children: false,
            level: 0,
        });
        app.update();
        app.update();

        assert!(!app.world().entity(fixture.parent).contains::<Expandable>());
        assert!(!app.world().entity(fixture.parent).contains::<Expanded>());
    }

    #[test]
    fn adding_a_row_to_an_existing_container_updates_visible_rows() {
        let (mut app, window) = tree_app();
        let tree = app
            .world_mut()
            .spawn((TreeView::default(), ChildOf(window)))
            .id();
        let parent = app
            .world_mut()
            .spawn((
                TreeItem {
                    has_children: true,
                    level: 0,
                },
                Expanded,
                ChildOf(tree),
            ))
            .id();
        let container = app
            .world_mut()
            .spawn((TreeItemChildren, ChildOf(parent)))
            .id();
        let child_a = app
            .world_mut()
            .spawn((TreeItem::default(), ChildOf(container)))
            .id();
        app.update();
        focus(&mut app, child_a);

        let child_b = app
            .world_mut()
            .spawn((TreeItem::default(), ChildOf(container)))
            .id();
        app.update();

        press_key(&mut app, KeyCode::ArrowDown, window);

        assert_eq!(app.world().resource::<InputFocus>().get(), Some(child_b));
    }
}
