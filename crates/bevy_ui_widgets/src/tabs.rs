use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    system::{Commands, Query, Res, ResMut},
    template::FromTemplate,
};
use bevy_input::{
    keyboard::{KeyCode, KeyboardInput},
    ButtonState,
};
use bevy_input_focus::{
    tab_navigation::TabIndex, FocusCause, FocusedInput, InputFocus, InputFocusVisible,
};
use bevy_picking::{events::PointerClick, pointer::PointerButton};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{InteractionDisabled, Selectable, Selected};

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
#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[require(AccessibilityNode(accesskit::Node::new(Role::TabList)), SelectedTab)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct TabList {
    /// The axis used by arrow-key navigation.
    pub orientation: ControlOrientation,
    /// Whether keyboard navigation requests selection immediately.
    pub activation: TabActivation,
}

impl Default for TabList {
    fn default() -> Self {
        Self {
            orientation: ControlOrientation::Horizontal,
            activation: TabActivation::default(),
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

/// Plugin that registers tab-list observers and derived state.
pub struct TabPlugin;

impl Plugin for TabPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(tablist_on_click)
            .add_observer(tab_on_key_input)
            .add_systems(PostUpdate, update_tablist_derived_state);
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
    if selection.0 != Some(tab) {
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
        KeyCode::Enter | KeyCode::Space => Navigation::Activate,
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

/// Derives per-tab state from each [`TabList`]'s [`SelectedTab`] and the current keyboard focus:
///
/// - [`Selected`] markers mirror a validated `SelectedTab` (the referenced entity must be an
///   enabled direct child; anything else counts as no selection).
/// - [`TabIndex`] follows the roving-tabindex pattern: one tab per list is focusable (the
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
        events::Pointer,
        pointer::{Location, PointerButton, PointerId},
    };
    use bevy_window::{PrimaryWindow, Window, WindowRef};

    #[derive(Resource, Default)]
    struct SelectionRequests(Vec<(Entity, Option<Entity>)>);

    fn tab_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((
            InputPlugin,
            InputFocusPlugin,
            InputDispatchPlugin,
            TabPlugin,
        ))
        .init_resource::<SelectionRequests>()
        .add_observer(
            |change: On<crate::ValueChange<Option<Entity>>>,
             mut requests: ResMut<SelectionRequests>| {
                requests.0.push((change.source, change.value));
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
        press_key(&mut app, KeyCode::Space, window);

        assert_eq!(
            app.world().resource::<SelectionRequests>().0,
            [(list, Some(tab)), (list, Some(tab))]
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
}
