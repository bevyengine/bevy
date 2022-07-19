//! [`NavMenu`] builders to convert into [`TreeMenu`].
//!
//! This module defines a bunch of "seed" bundles. Systems in [`crate::named`],
//! and [`crate::resolve`] will take the components
//! defined in those seeds and replace them by [`NavMenu`]s. It is necessary
//! for a few things:
//! * The [`active_child`](TreeMenu::active_child) field of `NavMenu`, which
//!   cannot be inferred without the [`Focusable`](crate::Focusable)s children of that menu
//! * Finding the [`Focusable`](crate::Focusable) specified in [`ParentName`]
//!
//! # Seed bundles
//!
//! Seed bundles are collections of components that will trigger various
//! pre-processing to create a [`TreeMenu`]. They are a combination of those
//! components:
//! * [`TreeMenuSeed`]: the base seed, which will be converted into a [`TreeMenu`]
//!   in [`crate::resolve::insert_tree_menus`].
//! * [`ParentName`], the *by-name* marker: marks a [`TreeMenuSeed`] as needing
//!   its [`focus_parent`](TreeMenuSeed::focus_parent) to be updated by
//!   [`crate::named::resolve_named_menus`] with the [`Focusable`](crate::Focusable) which
//!   [`Name`](https://docs.rs/bevy/0.8.0/bevy/core/struct.Name.html) matches
//!   the one in [`ParentName`]. If for whatever reason that update doesn't
//!   happen, [`crate::resolve::insert_tree_menus`] will panic.
//!
//! Those components are combined in the seed bundles. Which processing step is
//! applied to the [`TreeMenuSeed`] depends on which components it was inserted
//! with. The bundles are:
//! * [`MenuSeed`]: Creates a [`NavMenu`].
//! * [`NamedMenuSeed`]: Creates a [`NavMenu`] "reachable from" the
//!   [`Focusable`](crate::Focusable) named in the [`ParentName`].
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
//! The resolve_named/insert relationship should be upheld. Otherwise, root
//! NavMenus will spawn instead of NavMenus with parent with given name.
#![allow(unused_parens)]

use std::borrow::Cow;

use bevy_core::Name;
use bevy_ecs::{
    entity::Entity,
    prelude::{Bundle, Component},
};

use crate::resolve::TreeMenu;

/// Option of an option.
///
/// It's really just `Option<Option<T>>` with some semantic sparkled on top.
#[derive(Clone)]
pub(crate) enum FailableOption<T> {
    Uninit,
    None,
    Some(T),
}
impl<T> FailableOption<T> {
    fn into_opt(self) -> Option<Option<T>> {
        match self {
            Self::Some(t) => Some(Some(t)),
            Self::None => Some(None),
            Self::Uninit => None,
        }
    }
}
impl<T> From<Option<T>> for FailableOption<T> {
    fn from(option: Option<T>) -> Self {
        option.map_or(Self::None, Self::Some)
    }
}

/// An uninitialized [`TreeMenu`].
///
/// It is added through one of the bundles defined in this crate by the user,
/// and picked up by the [`crate::resolve::insert_tree_menus`] system to create the
/// actual [`TreeMenu`] handled by the resolution algorithm.
#[derive(Component, Clone)]
pub(crate) struct TreeMenuSeed {
    pub(crate) focus_parent: FailableOption<Entity>,
    menu: NavMenu,
}
impl TreeMenuSeed {
    /// Initialize a [`TreeMenu`] with given active child.
    ///
    /// (Menus without focusables are a programming error)
    pub(crate) fn with_child(self, active_child: Entity) -> TreeMenu {
        let TreeMenuSeed { focus_parent, menu } = self;
        let msg = "An initialized parent value";
        TreeMenu {
            focus_parent: focus_parent.into_opt().expect(msg),
            setting: menu,
            active_child,
        }
    }
}

/// Component to specify creation of a [`TreeMenu`] refering to their parent
/// focusable by [`Name`](https://docs.rs/bevy/0.8.0/bevy/core/struct.Name.html)
///
/// It is used in [`crate::named::resolve_named_menus`] to figure out the
/// `Entity` id of the named parent of the [`TreeMenuSeed`] and set its
/// `focus_parent` field.
#[derive(Component, Clone)]
pub(crate) struct ParentName(pub(crate) Name);

/// Component to add to [`NavMenu`] entities to propagate `T` to all
/// [`Focusable`](crate::Focusable) children of that menu.
#[derive(Component, Clone)]
pub(crate) struct NavMarker<T>(pub(crate) T);

/// A menu that isolate children [`Focusable`](crate::Focusable)s from other
/// focusables and specify navigation method within itself.
///
/// # Usage
///
/// A [`NavMenu`] can be used to:
/// * Prevent navigation from one specific submenu to another
/// * Specify if 2d navigation wraps around the screen, see
///   [`NavMenu::Wrapping2d`].
/// * Specify "scope menus" such that a
///   [`NavRequest::ScopeMove`](crate::NavRequest::ScopeMove) emitted when
///   the focused element is a [`Focusable`](crate::Focusable) nested within this `NavMenu`
///   will navigate this menu. See [`NavMenu::BoundScope`] and
///   [`NavMenu::WrappingScope`].
/// * Specify _submenus_ and specify from where those submenus are reachable.
/// * Add a specific component to all [`Focusable`](crate::Focusable)s in this menu. You must
///   first create a "seed" bundle with any of the [`NavMenu`] methods and then
///   call [`marking`](MenuSeed::marking) on it.
/// * Specify which entity will be the parents of this [`NavMenu`], see
///   [`NavMenu::reachable_from`] or [`NavMenu::reachable_from_named`] if you don't
///   have access to the [`Entity`](https://docs.rs/bevy/0.8.0/bevy/ecs/entity/struct.Entity.html)
///   for the parent [`Focusable`](crate::Focusable)
///
/// If you want to specify which [`Focusable`](crate::Focusable) should be
/// focused first when entering a menu, you should mark one of the children of
/// this menu with [`Focusable::dormant`](crate::Focusable::dormant).
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
/// 3. Focusables in 2d menus must have a `GlobalTransform`.
///
/// # Panics
///
/// Thankfully, programming errors are caught early and you'll probably get a
/// panic fairly quickly if you don't follow the invariants.
/// * Invariant (1) panics as soon as you add the menu without focusable
///   children.
/// * Invariant (2) panics if the focus goes into a menu loop.
#[derive(Clone, Debug, Copy, PartialEq)]
#[non_exhaustive]
pub enum NavMenu {
    /// Non-wrapping menu with 2d navigation.
    ///
    /// It is possible to move around this menu in all cardinal directions, the
    /// focus changes according to the physical position of the
    /// [`Focusable`](crate::Focusable) in it.
    ///
    /// If the player moves to a direction where there aren't any focusables,
    /// nothing will happen.
    Bound2d,

    /// Wrapping menu with 2d navigation.
    ///
    /// It is possible to move around this menu in all cardinal directions, the
    /// focus changes according to the physical position of the
    /// [`Focusable`](crate::Focusable) in it.
    ///
    /// If the player moves to a direction where there aren't any focusables,
    /// the focus will "wrap" to the other direction of the screen.
    Wrapping2d,

    /// Non-wrapping scope menu
    ///
    /// Controlled with [`NavRequest::ScopeMove`](crate::NavRequest::ScopeMove)
    /// even when the focused element is not in this menu, but in a submenu
    /// reachable from this one.
    BoundScope,

    /// Wrapping scope menu
    ///
    /// Controlled with [`NavRequest::ScopeMove`](crate::NavRequest::ScopeMove) even
    /// when the focused element is not in this menu, but in a submenu reachable from this one.
    WrappingScope,
}
impl NavMenu {
    pub(crate) fn bound(&self) -> bool {
        matches!(self, NavMenu::BoundScope | NavMenu::Bound2d)
    }
    pub(crate) fn is_2d(&self) -> bool {
        !self.is_scope()
    }
    pub(crate) fn is_scope(&self) -> bool {
        matches!(self, NavMenu::BoundScope | NavMenu::WrappingScope)
    }
}

/// A "seed" for creation of a [`NavMenu`].
///
/// Internally, `bevy_ui_navigation` uses a special component to mark UI nodes
/// as "menus", this tells the navigation algorithm to add that component to
/// this `Entity`.
#[derive(Bundle, Clone)]
pub struct MenuSeed {
    seed: TreeMenuSeed,
}

/// Bundle to specify creation of a [`NavMenu`] refering to their parent
/// focusable by [`Name`](https://docs.rs/bevy/0.8.0/bevy/core/struct.Name.html)
///
/// This is useful if, for example, you just want to spawn your UI without
/// keeping track of entity ids of buttons that leads to submenus.
#[derive(Bundle, Clone)]
pub struct NamedMenuSeed {
    seed: TreeMenuSeed,
    parent_name: ParentName,
}

impl NavMenu {
    fn seed(self, focus_parent: FailableOption<Entity>) -> TreeMenuSeed {
        TreeMenuSeed {
            focus_parent,
            menu: self,
        }
    }

    /// Spawn a [`NavMenu`] seed with provided parent entity (or root if
    /// `None`).
    ///
    /// Prefer [`Self::reachable_from`] and [`Self::root`] to this if you don't
    /// already have an `Option<Entity>`.
    pub fn with_parent(self, focus_parent: Option<Entity>) -> MenuSeed {
        let seed = self.seed(focus_parent.into());
        MenuSeed { seed }
    }

    /// Spawn this menu with no parents.
    ///
    /// No [`Focusable`](crate::Focusable) will "lead to" this menu. You either need to
    /// programmatically give focus to this menu tree with
    /// [`NavRequest::FocusOn`](crate::NavRequest::FocusOn) or have only one root menu.
    pub fn root(self) -> MenuSeed {
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
    /// to reach `NavMenu` X from `Focusable` Y if there is a path from
    /// `NavMenu` X to `Focusable` Y.
    pub fn reachable_from(self, focusable: Entity) -> MenuSeed {
        self.with_parent(Some(focusable))
    }

    /// Spawn this menu as reachable from a [`Focusable`](crate::Focusable) with a
    /// [`Name`](https://docs.rs/bevy/0.8.0/bevy/core/struct.Name.html)
    /// component.
    ///
    /// This is useful if, for example, you just want to spawn your UI without
    /// keeping track of entity ids of buttons that leads to submenus.
    pub fn reachable_from_named(self, parent_label: impl Into<Cow<'static, str>>) -> NamedMenuSeed {
        NamedMenuSeed {
            parent_name: ParentName(Name::new(parent_label)),
            seed: self.seed(FailableOption::Uninit),
        }
    }
}
