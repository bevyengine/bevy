//! This module provides a framework for handling linear tab-key navigation in Bevy applications.
//!
//! The rules of tabbing are derived from the HTML specification, and are as follows:
//!
//! * An index >= 0 means that the entity is tabbable via sequential navigation.
//!   The order of tabbing is determined by the index, with lower indices being tabbed first.
//!   If two entities have the same index, then the order is determined by the order of
//!   the entities in the ECS hierarchy (as determined by Parent/Child).
//! * An index < 0 means that the entity is not focusable via sequential navigation, but
//!   can still be focused via direct selection.
//!
//! Tabbable entities must be descendants of a [`TabGroup`] entity, which is a component that
//! marks a tree of entities as containing tabbable elements. The order of tab groups
//! is determined by the [`TabGroup::order`] field, with lower orders being tabbed first. Modal tab groups
//! are used for ui elements that should only tab within themselves, such as modal dialog boxes.
//!
//! To enable automatic tabbing, add the
//! [`TabNavigationPlugin`] and [`InputDispatchPlugin`](crate::InputDispatchPlugin) to your app.
//! This will install a keyboard event observer on the primary window which automatically handles
//! tab navigation for you.
//!
//! Alternatively, if you want to have more control over tab navigation, or are using an input-action-mapping framework,
//! you can use the [`TabNavigation`] system parameter directly instead.
//! This object can be injected into your systems, and provides a [`navigate`](`TabNavigation::navigate`) method which can be
//! used to navigate between focusable entities.

use alloc::vec::Vec;
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{With, Without},
    system::{Commands, Query, Res, ResMut, SystemParam},
};
use bevy_input::{
    keyboard::{KeyCode, KeyboardInput},
    ButtonInput, ButtonState,
};
use bevy_picking::events::{Pointer, Press};
use bevy_window::{PrimaryWindow, Window};
use log::warn;
use thiserror::Error;

use crate::{AcquireFocus, FocusedInput, InputFocus, InputFocusVisible};

#[cfg(feature = "bevy_reflect")]
use {
    bevy_ecs::prelude::ReflectComponent,
    bevy_reflect::{prelude::*, Reflect},
};

/// A component which indicates that an entity wants to participate in tab navigation.
///
/// Note that you must also add the [`TabGroup`] component to the entity's ancestor in order
/// for this component to have any effect.
#[derive(Debug, Default, Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Default, Component, PartialEq, Clone)
)]
pub struct TabIndex(pub i32);

/// A component used to mark a tree of entities as containing tabbable elements.
#[derive(Debug, Default, Component, Copy, Clone)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Default, Component, Clone)
)]
pub struct TabGroup {
    /// The order of the tab group relative to other tab groups.
    pub order: i32,

    /// Whether this is a 'modal' group. If true, then tabbing within the group (that is,
    /// if the current focus entity is a child of this group) will cycle through the children
    /// of this group. If false, then tabbing within the group will cycle through all non-modal
    /// tab groups.
    pub modal: bool,
}

impl TabGroup {
    /// Create a new tab group with the given order.
    pub fn new(order: i32) -> Self {
        Self {
            order,
            modal: false,
        }
    }

    /// Create a modal tab group.
    pub fn modal() -> Self {
        Self {
            order: 0,
            modal: true,
        }
    }
}

/// A navigation action that users might take to navigate your user interface in a cyclic fashion.
///
/// These values are consumed by the [`TabNavigation`] system param.
#[derive(Clone, Copy)]
pub enum NavAction {
    /// Navigate to the next focusable entity, wrapping around to the beginning if at the end.
    ///
    /// This is commonly triggered by pressing the Tab key.
    Next,
    /// Navigate to the previous focusable entity, wrapping around to the end if at the beginning.
    ///
    /// This is commonly triggered by pressing Shift+Tab.
    Previous,
    /// Navigate to the first focusable entity.
    ///
    /// This is commonly triggered by pressing Home.
    First,
    /// Navigate to the last focusable entity.
    ///
    /// This is commonly triggered by pressing End.
    Last,
}

/// An error that can occur during [tab navigation](crate::tab_navigation).
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum TabNavigationError {
    /// No tab groups were found.
    #[error("No tab groups found")]
    NoTabGroups,
    /// No focusable entities were found.
    #[error("No focusable entities found")]
    NoFocusableEntities,
    /// Could not navigate to the next focusable entity.
    ///
    /// This can occur if your tab groups are malformed.
    #[error("Failed to navigate to next focusable entity")]
    FailedToNavigateToNextFocusableEntity,
    /// No tab group for the current focus entity was found.
    #[error("No tab group found for currently focused entity {previous_focus}. Users will not be able to navigate back to this entity.")]
    NoTabGroupForCurrentFocus {
        /// The entity that was previously focused,
        /// and is missing its tab group.
        previous_focus: Entity,
        /// The new entity that will be focused.
        ///
        /// If you want to recover from this error, set [`InputFocus`] to this entity.
        new_focus: Entity,
    },
}

/// An injectable helper object that provides tab navigation functionality.
#[doc(hidden)]
#[derive(SystemParam)]
pub struct TabNavigation<'w, 's> {
    // Query for tab groups.
    tabgroup_query: Query<'w, 's, (Entity, &'static TabGroup, &'static Children)>,
    // Query for tab indices.
    tabindex_query: Query<
        'w,
        's,
        (Entity, Option<&'static TabIndex>, Option<&'static Children>),
        Without<TabGroup>,
    >,
    // Query for parents.
    parent_query: Query<'w, 's, &'static ChildOf>,
}

impl TabNavigation<'_, '_> {
    /// Navigate to the desired focusable entity.
    ///
    /// Change the [`NavAction`] to navigate in a different direction.
    /// Focusable entities are determined by the presence of the [`TabIndex`] component.
    ///
    /// If no focusable entities are found, then this function will return either the first
    /// or last focusable entity, depending on the direction of navigation. For example, if
    /// `action` is `Next` and no focusable entities are found, then this function will return
    /// the first focusable entity.
    pub fn navigate(
        &self,
        focus: &InputFocus,
        action: NavAction,
    ) -> Result<Entity, TabNavigationError> {
        // If there are no tab groups, then there are no focusable entities.
        if self.tabgroup_query.is_empty() {
            return Err(TabNavigationError::NoTabGroups);
        }

        // Start by identifying which tab group we are in. Mainly what we want to know is if
        // we're in a modal group.
        let tabgroup = focus.0.and_then(|focus_ent| {
            self.parent_query
                .iter_ancestors(focus_ent)
                .find_map(|entity| {
                    self.tabgroup_query
                        .get(entity)
                        .ok()
                        .map(|(_, tg, _)| (entity, tg))
                })
        });

        let navigation_result = self.navigate_in_group(tabgroup, focus, action);

        match navigation_result {
            Ok(entity) => {
                if focus.0.is_some() && tabgroup.is_none() {
                    Err(TabNavigationError::NoTabGroupForCurrentFocus {
                        previous_focus: focus.0.unwrap(),
                        new_focus: entity,
                    })
                } else {
                    Ok(entity)
                }
            }
            Err(e) => Err(e),
        }
    }

    fn navigate_in_group(
        &self,
        tabgroup: Option<(Entity, &TabGroup)>,
        focus: &InputFocus,
        action: NavAction,
    ) -> Result<Entity, TabNavigationError> {
        // List of all focusable entities found.
        let mut focusable: Vec<(Entity, TabIndex, usize)> =
            Vec::with_capacity(self.tabindex_query.iter().len());

        match tabgroup {
            Some((tg_entity, tg)) if tg.modal => {
                // We're in a modal tab group, then gather all tab indices in that group.
                if let Ok((_, _, children)) = self.tabgroup_query.get(tg_entity) {
                    for child in children.iter() {
                        self.gather_focusable(&mut focusable, *child, 0);
                    }
                }
            }
            _ => {
                // Otherwise, gather all tab indices in all non-modal tab groups.
                let mut tab_groups: Vec<(Entity, TabGroup)> = self
                    .tabgroup_query
                    .iter()
                    .filter(|(_, tg, _)| !tg.modal)
                    .map(|(e, tg, _)| (e, *tg))
                    .collect();
                // Stable sort by group order
                tab_groups.sort_by_key(|(_, tg)| tg.order);

                // Search group descendants
                tab_groups
                    .iter()
                    .enumerate()
                    .for_each(|(idx, (tg_entity, _))| {
                        self.gather_focusable(&mut focusable, *tg_entity, idx);
                    });
            }
        }

        if focusable.is_empty() {
            return Err(TabNavigationError::NoFocusableEntities);
        }

        // Sort by TabGroup and then TabIndex
        focusable.sort_by(|(_, a_tab_idx, a_group), (_, b_tab_idx, b_group)| {
            if a_group == b_group {
                a_tab_idx.cmp(b_tab_idx)
            } else {
                a_group.cmp(b_group)
            }
        });

        let index = focusable.iter().position(|e| Some(e.0) == focus.0);
        let count = focusable.len();
        let next = match (index, action) {
            (Some(idx), NavAction::Next) => (idx + 1).rem_euclid(count),
            (Some(idx), NavAction::Previous) => (idx + count - 1).rem_euclid(count),
            (None, NavAction::Next) | (_, NavAction::First) => 0,
            (None, NavAction::Previous) | (_, NavAction::Last) => count - 1,
        };
        match focusable.get(next) {
            Some((entity, _, _)) => Ok(*entity),
            None => Err(TabNavigationError::FailedToNavigateToNextFocusableEntity),
        }
    }

    /// Gather all focusable entities in tree order.
    fn gather_focusable(
        &self,
        out: &mut Vec<(Entity, TabIndex, usize)>,
        parent: Entity,
        tab_group_idx: usize,
    ) {
        if let Ok((entity, tabindex, children)) = self.tabindex_query.get(parent) {
            if let Some(tabindex) = tabindex {
                if tabindex.0 >= 0 {
                    out.push((entity, *tabindex, tab_group_idx));
                }
            }
            if let Some(children) = children {
                for child in children.iter() {
                    // Don't traverse into tab groups, as they are handled separately.
                    if self.tabgroup_query.get(*child).is_err() {
                        self.gather_focusable(out, *child, tab_group_idx);
                    }
                }
            }
        } else if let Ok((_, tabgroup, children)) = self.tabgroup_query.get(parent) {
            if !tabgroup.modal {
                for child in children.iter() {
                    self.gather_focusable(out, *child, tab_group_idx);
                }
            }
        }
    }
}

/// Observer which sets focus to the nearest ancestor that has tab index, using bubbling.
pub(crate) fn acquire_focus(
    mut acquire_focus: On<AcquireFocus>,
    focusable: Query<(), With<TabIndex>>,
    windows: Query<(), With<Window>>,
    mut focus: ResMut<InputFocus>,
) {
    // If the entity has a TabIndex
    if focusable.contains(acquire_focus.focused_entity) {
        // Stop and focus it
        acquire_focus.propagate(false);
        // Don't mutate unless we need to, for change detection
        if focus.0 != Some(acquire_focus.focused_entity) {
            focus.0 = Some(acquire_focus.focused_entity);
        }
    } else if windows.contains(acquire_focus.focused_entity) {
        // Stop and clear focus
        acquire_focus.propagate(false);
        // Don't mutate unless we need to, for change detection
        if focus.0.is_some() {
            focus.clear();
        }
    }
}

/// Plugin for navigating between focusable entities using keyboard input.
pub struct TabNavigationPlugin;

impl Plugin for TabNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_tab_navigation);
        app.add_observer(acquire_focus);
        app.add_observer(click_to_focus);
    }
}

fn setup_tab_navigation(mut commands: Commands, window: Query<Entity, With<PrimaryWindow>>) {
    for window in window.iter() {
        commands.entity(window).observe(handle_tab_navigation);
    }
}

fn click_to_focus(
    press: On<Pointer<Press>>,
    mut focus_visible: ResMut<InputFocusVisible>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    // Because `Pointer` is a bubbling event, we don't want to trigger an `AcquireFocus` event
    // for every ancestor, but only for the original entity. Also, users may want to stop
    // propagation on the pointer event at some point along the bubbling chain, so we need our
    // own dedicated event whose propagation we can control.
    if press.entity == press.original_event_target() {
        // Clicking hides focus
        if focus_visible.0 {
            focus_visible.0 = false;
        }
        // Search for a focusable parent entity, defaulting to window if none.
        if let Ok(window) = windows.single() {
            commands.trigger(AcquireFocus {
                focused_entity: press.entity,
                window,
            });
        }
    }
}

/// Observer function which handles tab navigation.
///
/// This observer responds to [`KeyCode::Tab`] events and Shift+Tab events,
/// cycling through focusable entities in the order determined by their tab index.
///
/// Any [`TabNavigationError`]s that occur during tab navigation are logged as warnings.
pub fn handle_tab_navigation(
    mut event: On<FocusedInput<KeyboardInput>>,
    nav: TabNavigation,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<InputFocusVisible>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    // Tab navigation.
    let key_event = &event.input;
    if key_event.key_code == KeyCode::Tab
        && key_event.state == ButtonState::Pressed
        && !key_event.repeat
    {
        let maybe_next = nav.navigate(
            &focus,
            if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                NavAction::Previous
            } else {
                NavAction::Next
            },
        );

        match maybe_next {
            Ok(next) => {
                event.propagate(false);
                focus.set(next);
                visible.0 = true;
            }
            Err(e) => {
                warn!("Tab navigation error: {e}");
                // This failure mode is recoverable, but still indicates a problem.
                if let TabNavigationError::NoTabGroupForCurrentFocus { new_focus, .. } = e {
                    event.propagate(false);
                    focus.set(new_focus);
                    visible.0 = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::SystemState;

    use super::*;

    #[test]
    fn test_tab_navigation() {
        let mut app = App::new();
        let world = app.world_mut();

        let tab_group_entity = world.spawn(TabGroup::new(0)).id();
        let tab_entity_1 = world.spawn((TabIndex(0), ChildOf(tab_group_entity))).id();
        let tab_entity_2 = world.spawn((TabIndex(1), ChildOf(tab_group_entity))).id();

        let mut system_state: SystemState<TabNavigation> = SystemState::new(world);
        let tab_navigation = system_state.get(world);
        assert_eq!(tab_navigation.tabgroup_query.iter().count(), 1);
        assert_eq!(tab_navigation.tabindex_query.iter().count(), 2);

        let next_entity =
            tab_navigation.navigate(&InputFocus::from_entity(tab_entity_1), NavAction::Next);
        assert_eq!(next_entity, Ok(tab_entity_2));

        let prev_entity =
            tab_navigation.navigate(&InputFocus::from_entity(tab_entity_2), NavAction::Previous);
        assert_eq!(prev_entity, Ok(tab_entity_1));

        let first_entity = tab_navigation.navigate(&InputFocus::default(), NavAction::First);
        assert_eq!(first_entity, Ok(tab_entity_1));

        let last_entity = tab_navigation.navigate(&InputFocus::default(), NavAction::Last);
        assert_eq!(last_entity, Ok(tab_entity_2));
    }

    #[test]
    fn test_tab_navigation_between_groups_is_sorted_by_group() {
        let mut app = App::new();
        let world = app.world_mut();

        let tab_group_1 = world.spawn(TabGroup::new(0)).id();
        let tab_entity_1 = world.spawn((TabIndex(0), ChildOf(tab_group_1))).id();
        let tab_entity_2 = world.spawn((TabIndex(1), ChildOf(tab_group_1))).id();

        let tab_group_2 = world.spawn(TabGroup::new(1)).id();
        let tab_entity_3 = world.spawn((TabIndex(0), ChildOf(tab_group_2))).id();
        let tab_entity_4 = world.spawn((TabIndex(1), ChildOf(tab_group_2))).id();

        let mut system_state: SystemState<TabNavigation> = SystemState::new(world);
        let tab_navigation = system_state.get(world);
        assert_eq!(tab_navigation.tabgroup_query.iter().count(), 2);
        assert_eq!(tab_navigation.tabindex_query.iter().count(), 4);

        let next_entity =
            tab_navigation.navigate(&InputFocus::from_entity(tab_entity_1), NavAction::Next);
        assert_eq!(next_entity, Ok(tab_entity_2));

        let prev_entity =
            tab_navigation.navigate(&InputFocus::from_entity(tab_entity_2), NavAction::Previous);
        assert_eq!(prev_entity, Ok(tab_entity_1));

        let first_entity = tab_navigation.navigate(&InputFocus::default(), NavAction::First);
        assert_eq!(first_entity, Ok(tab_entity_1));

        let last_entity = tab_navigation.navigate(&InputFocus::default(), NavAction::Last);
        assert_eq!(last_entity, Ok(tab_entity_4));

        let next_from_end_of_group_entity =
            tab_navigation.navigate(&InputFocus::from_entity(tab_entity_2), NavAction::Next);
        assert_eq!(next_from_end_of_group_entity, Ok(tab_entity_3));

        let prev_entity_from_start_of_group =
            tab_navigation.navigate(&InputFocus::from_entity(tab_entity_3), NavAction::Previous);
        assert_eq!(prev_entity_from_start_of_group, Ok(tab_entity_2));
    }
}
