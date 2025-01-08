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
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    observer::Trigger,
    query::{With, Without},
    system::{Commands, Query, Res, ResMut, SystemParam},
};
use bevy_hierarchy::{Children, HierarchyQueryExt, Parent};
use bevy_input::{
    keyboard::{KeyCode, KeyboardInput},
    ButtonInput, ButtonState,
};
use bevy_utils::tracing::warn;
use bevy_window::PrimaryWindow;

use crate::{FocusedInput, InputFocus, InputFocusVisible};

/// A component which indicates that an entity wants to participate in tab navigation.
///
/// Note that you must also add the [`TabGroup`] component to the entity's ancestor in order
/// for this component to have any effect.
#[derive(Debug, Default, Component, Copy, Clone)]
pub struct TabIndex(pub i32);

/// A component used to mark a tree of entities as containing tabbable elements.
#[derive(Debug, Default, Component, Copy, Clone)]
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

/// A navigation action for tabbing.
///
/// These values are consumed by the [`TabNavigation`] system param.
pub enum NavAction {
    /// Navigate to the next focusable entity, wrapping around to the beginning if at the end.
    Next,
    /// Navigate to the previous focusable entity, wrapping around to the end if at the beginning.
    Previous,
    /// Navigate to the first focusable entity.
    First,
    /// Navigate to the last focusable entity.
    Last,
}

/// An injectable helper object that provides tab navigation functionality.
#[doc(hidden)]
#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
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
    parent_query: Query<'w, 's, &'static Parent>,
}

impl TabNavigation<'_, '_> {
    /// Navigate to the next focusable entity.
    ///
    /// Focusable entities are determined by the presence of the [`TabIndex`] component.
    ///
    /// Arguments:
    /// * `focus`: The current focus entity, or `None` if no entity has focus.
    /// * `action`: Whether to select the next, previous, first, or last focusable entity.
    ///
    /// If no focusable entities are found, then this function will return either the first
    /// or last focusable entity, depending on the direction of navigation. For example, if
    /// `action` is `Next` and no focusable entities are found, then this function will return
    /// the first focusable entity.
    pub fn navigate(&self, focus: &InputFocus, action: NavAction) -> Option<Entity> {
        // If there are no tab groups, then there are no focusable entities.
        if self.tabgroup_query.is_empty() {
            warn!("No tab groups found");
            return None;
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

        if focus.0.is_some() && tabgroup.is_none() {
            warn!("No tab group found for focus entity. Users will not be able to navigate back to this entity.");
        }

        self.navigate_in_group(tabgroup, focus, action)
    }

    fn navigate_in_group(
        &self,
        tabgroup: Option<(Entity, &TabGroup)>,
        focus: &InputFocus,
        action: NavAction,
    ) -> Option<Entity> {
        // List of all focusable entities found.
        let mut focusable: Vec<(Entity, TabIndex)> =
            Vec::with_capacity(self.tabindex_query.iter().len());

        match tabgroup {
            Some((tg_entity, tg)) if tg.modal => {
                // We're in a modal tab group, then gather all tab indices in that group.
                if let Ok((_, _, children)) = self.tabgroup_query.get(tg_entity) {
                    for child in children.iter() {
                        self.gather_focusable(&mut focusable, *child);
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
                tab_groups.sort_by(compare_tab_groups);

                // Search group descendants
                tab_groups.iter().for_each(|(tg_entity, _)| {
                    self.gather_focusable(&mut focusable, *tg_entity);
                });
            }
        }

        if focusable.is_empty() {
            warn!("No focusable entities found");
            return None;
        }

        // Stable sort by tabindex
        focusable.sort_by(compare_tab_indices);

        let index = focusable.iter().position(|e| Some(e.0) == focus.0);
        let count = focusable.len();
        let next = match (index, action) {
            (Some(idx), NavAction::Next) => (idx + 1).rem_euclid(count),
            (Some(idx), NavAction::Previous) => (idx + count - 1).rem_euclid(count),
            (None, NavAction::Next) | (_, NavAction::First) => 0,
            (None, NavAction::Previous) | (_, NavAction::Last) => count - 1,
        };
        focusable.get(next).map(|(e, _)| e).copied()
    }

    /// Gather all focusable entities in tree order.
    fn gather_focusable(&self, out: &mut Vec<(Entity, TabIndex)>, parent: Entity) {
        if let Ok((entity, tabindex, children)) = self.tabindex_query.get(parent) {
            if let Some(tabindex) = tabindex {
                if tabindex.0 >= 0 {
                    out.push((entity, *tabindex));
                }
            }
            if let Some(children) = children {
                for child in children.iter() {
                    // Don't traverse into tab groups, as they are handled separately.
                    if self.tabgroup_query.get(*child).is_err() {
                        self.gather_focusable(out, *child);
                    }
                }
            }
        } else if let Ok((_, tabgroup, children)) = self.tabgroup_query.get(parent) {
            if !tabgroup.modal {
                for child in children.iter() {
                    self.gather_focusable(out, *child);
                }
            }
        }
    }
}

fn compare_tab_groups(a: &(Entity, TabGroup), b: &(Entity, TabGroup)) -> core::cmp::Ordering {
    a.1.order.cmp(&b.1.order)
}

// Stable sort which compares by tab index
fn compare_tab_indices(a: &(Entity, TabIndex), b: &(Entity, TabIndex)) -> core::cmp::Ordering {
    a.1 .0.cmp(&b.1 .0)
}

/// Plugin for navigating between focusable entities using keyboard input.
pub struct TabNavigationPlugin;

impl Plugin for TabNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_tab_navigation);
    }
}

fn setup_tab_navigation(mut commands: Commands, window: Query<Entity, With<PrimaryWindow>>) {
    for window in window.iter() {
        commands.entity(window).observe(handle_tab_navigation);
    }
}

/// Observer function which handles tab navigation.
///
/// This observer responds to [`KeyCode::Tab`] events and Shift+Tab events,
/// cycling through focusable entities in the order determined by their tab index.
pub fn handle_tab_navigation(
    mut trigger: Trigger<FocusedInput<KeyboardInput>>,
    nav: TabNavigation,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<InputFocusVisible>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    // Tab navigation.
    let key_event = &trigger.event().input;
    if key_event.key_code == KeyCode::Tab
        && key_event.state == ButtonState::Pressed
        && !key_event.repeat
    {
        let next = nav.navigate(
            &focus,
            if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                NavAction::Previous
            } else {
                NavAction::Next
            },
        );
        if next.is_some() {
            trigger.propagate(false);
            focus.0 = next;
            visible.0 = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::system::SystemState;
    use bevy_hierarchy::BuildChildren;

    use super::*;

    #[test]
    fn test_tab_navigation() {
        let mut app = App::new();
        let world = app.world_mut();

        let tab_entity_1 = world.spawn(TabIndex(0)).id();
        let tab_entity_2 = world.spawn(TabIndex(1)).id();
        let mut tab_group_entity = world.spawn(TabGroup::new(0));
        tab_group_entity.replace_children(&[tab_entity_1, tab_entity_2]);

        let mut system_state: SystemState<TabNavigation> = SystemState::new(world);
        let tab_navigation = system_state.get(world);
        assert_eq!(tab_navigation.tabgroup_query.iter().count(), 1);
        assert_eq!(tab_navigation.tabindex_query.iter().count(), 2);

        let next_entity =
            tab_navigation.navigate(&InputFocus::from_entity(tab_entity_1), NavAction::Next);
        assert_eq!(next_entity, Some(tab_entity_2));

        let prev_entity =
            tab_navigation.navigate(&InputFocus::from_entity(tab_entity_2), NavAction::Previous);
        assert_eq!(prev_entity, Some(tab_entity_1));

        let first_entity = tab_navigation.navigate(&InputFocus::default(), NavAction::First);
        assert_eq!(first_entity, Some(tab_entity_1));

        let last_entity = tab_navigation.navigate(&InputFocus::default(), NavAction::Last);
        assert_eq!(last_entity, Some(tab_entity_2));
    }
}
