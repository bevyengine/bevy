//! [`MenuSetting`] builders to convert into [`TreeMenu`].
//!
//! # Ordering
//!
//! In order to correctly create the [`TreeMenu`] specified with the bundles
//! declared int this module, the systems need to be ran in this order:
//!
//! ```text
//! named::resolve_named_menus → resolve::insert_tree_menus
//! ```
//!
//! The `resolve_named/insert` relationship should be upheld. Otherwise, root
//! [`MenuSetting`]s will spawn instead of [`MenuSetting`]s with parent with given name.
//!
//! [`Focusable`]: crate::Focusable

use std::borrow::Cow;

use bevy_core::Name;
use bevy_ecs::{
    entity::Entity,
    prelude::{Bundle, Component},
};
use bevy_reflect::Reflect;

#[derive(Component, Debug, Clone)]
pub enum MenuBuilder {
    /// Component to specify creation of a [`TreeMenu`] refering to their parent
    /// focusable by [`Name`].
    ///
    /// This is useful if, for example, you just want to spawn your UI without
    /// keeping track of entity ids of buttons that leads to submenus.
    NamedParent(Name),
    EntityParent(Entity),
    Root,
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

/// A builder for creation of a [`MenuSetting`].
///
/// Internally, `bevy_ui_navigation` uses a special component to mark UI nodes
/// as "menus", this tells the navigation algorithm to add that component to
/// this `Entity`.
#[derive(Bundle)]
pub struct MenuBuilderBundle {
    pub focus_parent: MenuBuilder,
    pub settings: MenuSetting,
}

/// A menu that isolate children [`Focusable`](crate::Focusable)s from other
/// focusables and specify navigation method within itself.
///
/// # Usage
///
/// A [`MenuSetting`] can be used to:
/// * Prevent navigation from one specific submenu to another
/// * Specify if 2d navigation wraps around the screen, see
///   [`MenuSetting::Wrapping2d`].
/// * Specify "scope menus" such that a
///   [`NavRequest::ScopeMove`](crate::NavRequest::ScopeMove) emitted when
///   the focused element is a [`Focusable`](crate::Focusable) nested within this `MenuSetting`
///   will navigate this menu. See [`MenuSetting::BoundScope`] and
///   [`MenuSetting::WrappingScope`].
/// * Specify _submenus_ and specify from where those submenus are reachable.
/// * Add a specific component to all [`Focusable`](crate::Focusable)s in this menu. You must
///   first create a "seed" bundle with any of the [`MenuSetting`] methods and then
///   call [`marking`](MenuSeed::marking) on it.
/// * Specify which entity will be the parents of this [`MenuSetting`], see
///   [`MenuSetting::reachable_from`] or [`MenuSetting::reachable_from_named`] if you don't
///   have access to the [`Entity`]
///   for the parent [`Focusable`](crate::Focusable)
///
/// If you want to specify which [`Focusable`](crate::Focusable) should be
/// focused first when entering a menu, you should mark one of the children of
/// this menu with [`Focusable::prioritized`](crate::Focusable::prioritized).
///
/// ## Example
///
/// See the example in this [crate]'s root level documentation page.
///
/// # Invariants
///
/// **You need to follow those rules (invariants) to avoid panics**:
/// 1. A `Menu` must have **at least one** [`Focusable`](crate::Focusable) child in the UI
///    hierarchy.
/// 2. There must not be a menu loop. Ie: a way to go from menu A to menu B and
///    then from menu B to menu A while never going back.
///
/// # Panics
///
/// Thankfully, programming errors are caught early and you'll probably get a
/// panic fairly quickly if you don't follow the invariants.
/// * Invariant (1) panics as soon as you add the menu without focusable
///   children.
/// * Invariant (2) panics if the focus goes into a menu loop.
#[derive(Clone, Default, Component, Debug, Copy, PartialEq, Reflect)]
pub struct MenuSetting {
    /// Whether to wrap navigation.
    ///
    /// When the player moves to a direction where there aren't any focusables,
    /// if this is true, the focus will "wrap" to the other direction of the screen.
    pub wrapping: bool,
    /// Whether this is a scope menu.
    ///
    /// A scope menu is controlled with [`NavRequest::ScopeMove`](crate::NavRequest::ScopeMove)
    /// even when the focused element is not in this menu, but in a submenu
    /// reachable from this one.
    pub scope: bool,
}
impl MenuSetting {
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

impl MenuSetting {
    fn seed(self, focus_parent: MenuBuilder) -> MenuBuilderBundle {
        MenuBuilderBundle {
            focus_parent,
            settings: self,
        }
    }

    /// Spawn a [`MenuSetting`] seed with provided parent entity (or root if
    /// `None`).
    ///
    /// Prefer [`Self::reachable_from`] and [`Self::root`] to this if you don't
    /// already have an `Option<Entity>`.
    pub fn with_parent(self, focus_parent: Option<Entity>) -> MenuBuilderBundle {
        self.seed(focus_parent.into())
    }

    /// Spawn this menu with no parents.
    ///
    /// No [`Focusable`](crate::Focusable) will "lead to" this menu. You either need to
    /// programmatically give focus to this menu tree with
    /// [`NavRequest::FocusOn`](crate::NavRequest::FocusOn) or have only one root menu.
    pub fn root(self) -> MenuBuilderBundle {
        self.with_parent(None)
    }

    /// Spawn this menu as reachable from a given [`Focusable`](crate::Focusable)
    ///
    /// When requesting [`NavRequest::Action`](crate::NavRequest::Action)
    /// when `focusable` is focused, the focus will be changed to a focusable
    /// within this menu.
    ///
    /// # Important
    ///
    /// You must ensure this doesn't create a cycle. Eg: you shouldn't be able
    /// to reach `MenuSetting` X from `Focusable` Y if there is a path from
    /// `MenuSetting` X to `Focusable` Y.
    pub fn reachable_from(self, focusable: Entity) -> MenuBuilderBundle {
        self.with_parent(Some(focusable))
    }

    /// Spawn this menu as reachable from a [`Focusable`](crate::Focusable) with a
    /// [`Name`] component.
    ///
    /// This is useful if, for example, you just want to spawn your UI without
    /// keeping track of entity ids of buttons that leads to submenus.
    ///
    /// # Important
    ///
    /// You must ensure this doesn't create a cycle. Eg: you shouldn't be able
    /// to reach `MenuSetting` X from `Focusable` Y if there is a path from
    /// `MenuSetting` X to `Focusable` Y.
    pub fn reachable_from_named(
        self,
        parent_label: impl Into<Cow<'static, str>>,
    ) -> MenuBuilderBundle {
        MenuBuilderBundle {
            focus_parent: MenuBuilder::NamedParent(Name::new(parent_label)),
            settings: self,
        }
    }
}
