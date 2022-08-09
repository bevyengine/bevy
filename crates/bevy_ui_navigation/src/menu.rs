//! Contains menu-related components.

use std::borrow::Cow;

use bevy_core::Name;
use bevy_ecs::{
    entity::Entity,
    prelude::Component,
    query::{Changed, With},
    reflect::ReflectComponent,
    system::{Commands, Query},
};
use bevy_log::warn;
use bevy_reflect::Reflect;

use crate::resolve::{NavQueries, TreeMenu};
use crate::{
    focusable::{FocusState, Focusable},
    resolve::ChildQueries,
};

/// Tell the navigation system to turn this UI node into a menu.
///
/// Note that `MenuBuilder` is replaced by a private component when encoutered.
#[derive(Component, Debug, Clone)]
pub enum MenuBuilder {
    /// Create a menu as reachable from a [`Focusable`]
    /// with a [`Name`] component.
    ///
    /// This is useful if, for example, you just want to spawn your UI without
    /// keeping track of entity ids of buttons that leads to submenus.
    ///
    /// See [`MenuBuilder::from_named`] for an easier to use method
    /// if you don't have a [`Name`] ready to use.
    ///
    /// # Important
    ///
    /// You must ensure this doesn't create a cycle. Eg: you shouldn't be able
    /// to reach `MenuSetting` X from [`Focusable`] Y if there is a path from
    /// `MenuSetting` X to `Focusable` Y.
    ///
    /// [`Focusable`]: crate::prelude::Focusable
    NamedParent(Name),

    /// Create a menu as reachable from a given [`Focusable`].
    ///
    /// When requesting [`NavRequest::Action`] when `Entity` is focused,
    /// the focus will be changed to a focusable within this menu.
    ///
    /// # Important
    ///
    /// You must ensure this doesn't create a cycle. Eg: you shouldn't be able
    /// to reach `MenuSetting` X from `Focusable` Y if there is a path from
    /// `MenuSetting` X to `Focusable` Y.
    ///
    /// [`Focusable`]: crate::prelude::Focusable
    /// [`NavRequest::Action`]: crate::prelude::NavRequest::Action
    EntityParent(Entity),

    /// Create a menu with no parents.
    ///
    /// No [`Focusable`] will "lead to" this menu.
    /// You either need to programmatically give focus to this menu tree
    /// with [`NavRequest::FocusOn`]
    /// or have only one root menu.
    ///
    /// [`Focusable`]: crate::prelude::Focusable
    /// [`NavRequest::FocusOn`]: crate::prelude::NavRequest::FocusOn
    Root,
}
impl MenuBuilder {
    /// Create a [`MenuBuilder::NamedParent`] directly from `String` or `&str`.
    pub fn from_named(parent: impl Into<Cow<'static, str>>) -> Self {
        Self::NamedParent(Name::new(parent))
    }
}
impl From<Option<Entity>> for MenuBuilder {
    fn from(parent: Option<Entity>) -> Self {
        match parent {
            Some(parent) => MenuBuilder::EntityParent(parent),
            None => MenuBuilder::Root,
        }
    }
}
impl TryFrom<&MenuBuilder> for Option<Entity> {
    type Error = ();
    fn try_from(value: &MenuBuilder) -> Result<Self, Self::Error> {
        match value {
            MenuBuilder::EntityParent(parent) => Ok(Some(*parent)),
            MenuBuilder::NamedParent(_) => Err(()),
            MenuBuilder::Root => Ok(None),
        }
    }
}

/// A menu that isolate children [`Focusable`]s from other focusables
/// and specify navigation method within itself.
///
/// # Usage
///
/// A `MenuSetting` can be used to:
/// * Prevent navigation from one specific submenu to another
/// * Specify if 2d navigation wraps around the screen,
///   see [`MenuSetting::wrapping`].
/// * Specify "scope menus" such that sending a [`NavRequest::ScopeMove`]
///   when the focused element is a [`Focusable`] nested within this `MenuSetting`
///   will move cursor within this menu.
///   See [`MenuSetting::scope`].
/// * Specify _submenus_ and specify from where those submenus are reachable.
/// * Specify which entity will be the parents of this [`MenuSetting`].
///   See [`MenuBuilder`].
///
/// If you want to specify which [`Focusable`] should be focused first
/// when entering a menu,
/// you should mark one of the children of this menu with [`Focusable::prioritized`].
///
/// # Limitations
///
/// Menu navigation relies heavily on the bevy hierarchy being consistent.
/// You might get inconsistent state under the folowing conditions:
///
/// - You despawned an entity in the [`FocusState::Active`] state
/// - You changed the parent of a [`Focusable`] member of a menu to a new menu.
///         
/// The navigation system might still work as expected,
/// however, [`Focusable::state`] may be missleading
/// for the length of one frame.
///
/// # Panics
///
/// **Menu loops will cause a panic**.
/// A menu loop is a way to go from menu A to menu B and
/// then from menu B to menu A while never going back.
///
/// Don't worry though, menu loops are really hard to make by accident,
/// and it will only panic if you use a `NavRequest::FocusOn(entity)`
/// where `entity` is inside a menu loop.
///
/// [`NavRequest`]: crate::prelude::NavRequest
/// [`Focusable`]: crate::prelude::Focusable
/// [`FocusState::Active`]: crate::prelude::FocusState::Active
/// [`Focusable::state`]: crate::prelude::Focusable::state
/// [`Focusable::prioritized`]: crate::prelude::Focusable::prioritized
/// [`NavRequest::ScopeMove`]: crate::prelude::NavRequest::ScopeMove
/// [`NavRequest`]: crate::prelude::NavRequest
#[derive(Clone, Default, Component, Debug, Copy, PartialEq, Reflect)]
#[reflect(Component)]
pub struct MenuSetting {
    /// Whether to wrap navigation.
    ///
    /// When the player moves to a direction where there aren't any focusables,
    /// if this is true, the focus will "wrap" to the other direction of the screen.
    pub wrapping: bool,

    /// Whether this is a scope menu.
    ///
    /// A scope menu is controlled with [`NavRequest::ScopeMove`]
    /// even when the focused element is not in this menu, but in a submenu
    /// reachable from this one.
    ///
    /// [`NavRequest::ScopeMove`]: crate::prelude::NavRequest::ScopeMove
    pub scope: bool,
}
impl MenuSetting {
    /// New default settings.
    pub fn new() -> Self {
        Self::default()
    }
    /// Set wrapping.
    ///
    /// When the player moves to a direction where there aren't any focusables,
    /// if this is true, the focus will "wrap" to the other direction of the screen.
    pub fn wrap(mut self) -> Self {
        self.wrapping = true;
        self
    }
    /// Set scope.
    ///
    /// A scope menu is controlled with [`NavRequest::ScopeMove`]
    /// even when the focused element is not in this menu, but in a submenu
    /// reachable from this one.
    ///
    /// [`NavRequest::ScopeMove`]: crate::prelude::NavRequest::ScopeMove
    pub fn scope(mut self) -> Self {
        self.scope = true;
        self
    }
    pub(crate) fn bound(&self) -> bool {
        !self.wrapping
    }
    pub(crate) fn is_2d(&self) -> bool {
        !self.is_scope()
    }
    pub(crate) fn is_scope(&self) -> bool {
        self.scope
    }
}

/// Replaces [`MenuBuilder`]s with proper [`TreeMenu`]s.
pub(crate) fn insert_tree_menus(
    mut commands: Commands,
    builders: Query<(Entity, &MenuBuilder), With<MenuSetting>>,
    queries: NavQueries,
) {
    use FocusState::{Active, Focused, Prioritized};

    let no_focusables = "When spawning a menu, the menu must at least have \
        one (transitive) child entity with a Focusable component";
    let mut inserts = Vec::new();
    for (entity, builder) in &builders {
        let children = queries.children.focusables_of(entity);
        let child = children
            .iter()
            .find_map(|e| {
                let (_, focusable) = queries.focusables.get(*e).ok()?;
                matches!(focusable.state, Prioritized | Active | Focused).then_some(e)
            })
            .or_else(|| children.first())
            .expect(no_focusables);
        if let Ok(focus_parent) = builder.try_into() {
            let menu = TreeMenu {
                focus_parent,
                active_child: *child,
            };
            inserts.push((entity, (menu,)));
        } else {
            warn!("Encountered a non-translated named menu builder");
        }
        commands.entity(entity).remove::<MenuBuilder>();
    }
    commands.insert_or_spawn_batch(inserts);
}

/// Make sure menus pointing to an active child do not point to a blocked focusable.
pub(crate) fn consistent_menu(
    updated_focusables: Query<(Entity, &Focusable), Changed<Focusable>>,
    children: ChildQueries,
    mut menus: Query<(Entity, &mut TreeMenu)>,
) {
    for (entity, updated) in &updated_focusables {
        if updated.state() != FocusState::Blocked {
            continue;
        }
        for (menu_entity, mut menu) in &mut menus {
            if menu.active_child != entity {
                continue;
            }
            if let Some(new_active) = children.focusables_of(menu_entity).first().copied() {
                menu.active_child = new_active;
            }
            // We found the unique menu that leads to the changed entity
            // continue to check for next changed focusable.
            break;
        }
    }
}
