//! Navigation events and requests.
//!
//! The navigation system works through bevy's `Events` system. Basically, it is
//! a system with one input and two outputs:
//! * Input [`Events<NavRequst>`](https://docs.rs/bevy/0.8.0/bevy/app/struct.Events.html),
//!   tells the navigation system what to do. Your app should have a system
//!   that writes to a [`EventWriter<NavRequest>`](https://docs.rs/bevy/0.8.0/bevy/app/struct.EventWriter.html)
//!   based on inputs or internal game state. Usually, the default input systems specified
//!   in [`crate::systems`] do that for you. But you can add your own requests
//!   on top of the ones the default systems send. For example to unlock the UI with
//!   [`NavRequest::Free`].
//! * Output [`Focusable`](crate::Focusable) components. The navigation system
//!   updates the focusables component according to the focus state of the
//!   navigation system. See examples directory for how to read those
//! * Output [`EventReader<NavEvent>`](https://docs.rs/bevy/0.8.0/bevy/app/struct.EventReader.html),
//!   contains specific information about what the navigation system is doing.
use bevy_ecs::entity::Entity;
use non_empty_vec::NonEmpty;

/// Requests to send to the navigation system to update focus.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NavRequest {
    /// Move in 2d in provided direction.
    Move(Direction),
    /// Move within the encompassing [`NavMenu::BoundScope`](crate::NavMenu::BoundScope).
    ScopeMove(ScopeDirection),
    /// Enter submenu if any [`NavMenu::reachable_from`](crate::NavMenu::reachable_from)
    /// the currently focused entity.
    Action,
    /// Leave this submenu to enter the one it is [`reachable_from`](crate::NavMenu::reachable_from).
    Cancel,
    /// Move the focus to any arbitrary [`Focusable`](crate::Focusable) entity.
    FocusOn(Entity),
    /// Unlocks the navigation system.
    ///
    /// A [`NavEvent::Unlocked`] will be emitted as a response if the
    /// navigation system was indeed locked.
    Free,
}

/// Direction for movement in [`NavMenu::BoundScope`](crate::NavMenu::BoundScope) menus.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ScopeDirection {
    Next,
    Previous,
}

/// 2d direction to move in normal menus
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Direction {
    South,
    North,
    East,
    West,
}

/// Events emitted by the navigation system.
///
/// Useful if you want to react to [`NavEvent::NoChanges`] event, for example
/// when a "start game" button is focused and the [`NavRequest::Action`] is
/// pressed.
#[derive(Debug, Clone)]
pub enum NavEvent {
    /// Tells the app which element is the first one to be focused.
    ///
    /// This will be sent whenever the number of focused elements go from 0 to 1.
    /// Meaning: whenever you spawn a new UI with [`crate::Focusable`] elements.
    InitiallyFocused(Entity),

    /// Focus changed
    ///
    /// ## Notes
    ///
    /// Both `to` and `from` are ascending, meaning that the focused and newly
    /// focused elements are the first of their respective vectors.
    ///
    /// [`NonEmpty`] enables you to safely check `to.first()` or `from.first()`
    /// without returning an option. It is guaranteed that there is at least
    /// one element.
    FocusChanged {
        /// The list of elements that has become active after the focus
        /// change
        to: NonEmpty<Entity>,
        /// The list of active elements from the focused one to the last
        /// active which is affected by the focus change
        from: NonEmpty<Entity>,
    },
    /// The [`NavRequest`] didn't lead to any change in focus.
    NoChanges {
        /// The list of active elements from the focused one to the last
        /// active which is affected by the focus change
        from: NonEmpty<Entity>,
        /// The [`NavRequest`] that didn't do anything
        request: NavRequest,
    },
    /// A [lock focusable](crate::Focusable::lock) has been triggered
    ///
    /// Once the navigation plugin enters a locked state, the only way to exit
    /// it is to send a [`NavRequest::Free`].
    Locked(Entity),

    /// A [lock focusable](crate::Focusable::lock) has been triggered
    ///
    /// Once the navigation plugin enters a locked state, the only way to exit
    /// it is to send a [`NavRequest::Free`].
    Unlocked(Entity),
}
impl NavEvent {
    /// Convenience function to construct a `FocusChanged` with a single `to`
    ///
    /// Usually the `NavEvent::FocusChanged.to` field has a unique value.
    pub(crate) fn focus_changed(to: Entity, from: NonEmpty<Entity>) -> NavEvent {
        NavEvent::FocusChanged {
            from,
            to: NonEmpty::new(to),
        }
    }
}
