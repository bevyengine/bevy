use bevy_ecs::prelude::Component;
use bevy_reflect::prelude::*;

/// State of a [`Focusable`].
#[derive(Copy, Clone, PartialEq, Eq, Debug, Reflect)]
#[reflect_value(PartialEq)]
pub enum FocusState {
    /// An entity that was previously [`FocusState::Active`]
    /// from a branch of the menu tree that is currently not _focused_.
    /// When focus comes back to the [`MenuSetting`] containing this [`Focusable`],
    /// the `Prioritized` element will be the [`FocusState::Focused`] entity.
    ///
    /// [`MenuSetting`]: crate::menu::MenuSetting
    Prioritized,

    /// The currently highlighted/used entity,
    /// there is only a signle _focused_ entity.
    ///
    /// All navigation requests start from it.
    ///
    /// To set an arbitrary [`Focusable`] to _focused_, you should send a
    /// [`NavRequest::FocusOn`] request.
    ///
    /// [`NavRequest::FocusOn`]: crate::events::NavRequest::FocusOn
    Focused,

    /// This [`Focusable`] is on the path in the menu tree
    /// to the current [`FocusState::Focused`] entity.
    ///
    /// [`FocusState::Active`] focusables are the [`Focusable`]s
    /// from previous menus that were activated
    /// in order to reach the [`MenuSetting`] containing
    /// the currently _focused_ element.
    ///
    /// It's the "breadcrumb" of buttons to activate to reach
    /// the currently focused element from the root menu.
    ///
    /// [`MenuSetting`]: crate::menu::MenuSetting
    Active,

    /// Prevents all interactions with this [`Focusable`].
    ///
    /// This is equivalent to removing the `Focusable` component
    /// from the entity, but without the latency.
    Blocked,

    /// None of the above:
    /// This [`Focusable`] is neither `Prioritized`, `Focused` or `Active`.
    Inert,
}

/// The actions triggered by a [`Focusable`].
#[derive(Copy, Clone, PartialEq, Eq, Default, Debug, Reflect)]
#[reflect_value(PartialEq)]
#[non_exhaustive]
pub enum FocusAction {
    /// Acts like a standard navigation node.
    ///
    /// Goes into relevant menu if any [`MenuSetting`] is
    /// _[reachable from]_ this [`Focusable`].
    ///
    /// [`MenuSetting`]: crate::menu::MenuSetting
    /// [reachable from]: crate::menu::MenuBuilder::from_named
    #[default]
    Normal,

    /// If we receive [`NavRequest::Action`]
    /// while this [`Focusable`] is focused,
    /// it will act as a [`NavRequest::Cancel`]
    /// (leaving submenu to enter the parent one).
    ///
    /// [`NavRequest::Action`]: crate::events::NavRequest::Action
    /// [`NavRequest::Cancel`]: crate::events::NavRequest::Cancel
    Cancel,

    /// If we receive [`NavRequest::Action`]
    /// while this [`Focusable`] is focused,
    /// the navigation system will freeze
    /// until [`NavRequest::Unlock`] is received,
    /// sending a [`NavEvent::Unlocked`].
    ///
    /// This is useful to implement widgets with complex controls
    /// you don't want to accidentally unfocus,
    /// or suspending the navigation system while in-game.
    ///
    /// [`NavRequest::Action`]: crate::events::NavRequest::Action
    /// [`NavRequest::Unlock`]: crate::events::NavRequest::Unlock
    /// [`NavEvent::Unlocked`]: crate::events::NavEvent::Unlocked
    Lock,
}

/// An [`Entity`] that can be navigated to, using the cursor navigation system.
///
/// It is in one of multiple [`FocusState`],
/// you can check its state with the [`Focusable::state`] method.
///
/// A `Focusable` can execute a variety of [`FocusAction`]
/// when receiving [`NavRequest::Action`],
/// the default one is [`FocusAction::Normal`].
///
/// **Note**: If you are using [menu navigation],
/// you should avoid updating manually the state of [`Focusable`]s.
/// You should instead use [`NavRequest`] to manipulate and change focus.
///
/// [`NavRequest`]: crate::events::NavRequest
/// [`NavRequest::Action`]: crate::events::NavRequest::Action
/// [`Entity`]: bevy_ecs::prelude::Entity
/// [menu navigation]: crate::menu
#[derive(Component, Clone, Debug, Reflect)]
pub struct Focusable {
    pub(crate) state: FocusState,
    pub(crate) action: FocusAction,
}
impl Default for Focusable {
    fn default() -> Self {
        Focusable {
            state: FocusState::Inert,
            action: FocusAction::Normal,
        }
    }
}
impl Focusable {
    /// Default Focusable.
    pub fn new() -> Self {
        Self::default()
    }

    /// The [`FocusState`] of this `Focusable`.
    pub fn state(&self) -> FocusState {
        self.state
    }
    /// The [`FocusAction`] of this `Focusable`.
    pub fn action(&self) -> FocusAction {
        self.action
    }
    /// A "cancel" focusable, see [`FocusAction::Cancel`].
    pub fn cancel() -> Self {
        Focusable {
            state: FocusState::Inert,
            action: FocusAction::Cancel,
        }
    }
    /// A "lock" focusable, see [`FocusAction::Lock`].
    pub fn lock() -> Self {
        Focusable {
            state: FocusState::Inert,
            action: FocusAction::Lock,
        }
    }
    /// A focusable that will get highlighted in priority when none are set yet.
    ///
    /// **WARNING**: Only use this when creating the UI.
    /// Any of the following state is unspecified
    /// and will likely result in broken behavior:
    /// * Having multiple prioritized `Focusable`s in the same menu.
    /// * Updating an already existing `Focusable` with this.
    ///
    /// # Usage
    ///
    /// ```rust
    /// use bevy_ui_navigation::focusable::Focusable;
    /// Focusable::new().prioritized();
    /// ```
    pub fn prioritized(self) -> Self {
        Self {
            state: FocusState::Prioritized,
            ..self
        }
    }

    /// A [`FocusState::Blocked`] focusable.
    ///
    /// This focusable will not be able to take focus until
    /// [`Focusable::unblock`] is called on it.
    ///
    /// # Usage
    ///
    /// ```rust
    /// use bevy_ui_navigation::focusable::Focusable;
    /// Focusable::new().blocked();
    /// ```
    pub fn blocked(self) -> Self {
        Self {
            state: FocusState::Blocked,
            ..self
        }
    }

    /// Prevent this [`Focusable`] from gaining focus until it is unblocked.
    ///
    /// **Note**: Due to the way focus is handled, this does nothing
    /// when the [`Focusable::state`] is [`FocusState::Active`]
    /// or [`FocusState::Focused`].
    ///
    /// Returns `true` if `self` has succesfully been blocked
    /// (its [`Focusable::state`] was either `Inert` or `Prioritized`).
    ///
    /// # Limitations
    ///
    /// - If all the children of a menu are blocked, when activating the menu's
    ///   parent, the block state of the last active focusable will be ignored.
    /// - When `FocusOn` to an focusable in a menu reachable from an blocked
    ///   focusable, its block state will be ignored.
    pub fn block(&mut self) -> bool {
        use FocusState::{Blocked, Inert, Prioritized};
        let blockable = matches!(self.state(), Inert | Prioritized);
        if blockable {
            self.state = Blocked;
        }
        blockable
    }

    /// Allow this [`Focusable`] to gain focus again,
    /// setting it to [`FocusState::Inert`].
    ///
    /// Returns `true` if `self`'s state was [`FocusState::Blocked`].
    pub fn unblock(&mut self) -> bool {
        let should_unblock = self.state == FocusState::Blocked;
        if should_unblock {
            self.state = FocusState::Inert;
        }
        should_unblock
    }
}

/// The currently _focused_ [`Focusable`].
///
/// You cannot edit it or create new `Focused` component.
/// To set an arbitrary [`Focusable`] to _focused_,
/// you should send [`NavRequest::FocusOn`].
///
/// This [`Component`] is useful
/// if you need to query for the _currently focused_ element,
/// using `Query<Entity, With<Focused>>` for example.
///
/// If a [`Focusable`] is focused,
/// its [`Focusable::state()`] will be [`FocusState::Focused`],
///
/// # Notes
///
/// The `Focused` marker component is only updated
/// at the end of the `CoreStage::Update` stage.
/// This means it might lead to a single frame of latency
/// compared to using [`Focusable::state()`].
///
/// [`NavRequest::FocusOn`]: crate::events::NavRequest::FocusOn
#[derive(Component)]
#[component(storage = "SparseSet")]
#[non_exhaustive]
pub struct Focused;
