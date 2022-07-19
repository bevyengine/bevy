//! Helpers for reacting to [`NavEvent`]s.
//!
//! This module defines [`SystemParam`] with methods that helps structure code
//! reacting to [`NavEvent`]s.
//!
//! The goal is to provide a conceptually simple abstraction over the specifics
//! of `bevy_ui_navigation` while preserving access to the specifics for the
//! rare occurences when it is needed.
//!
//! # Motivation
//!
//! The default way of hooking your UI to your code in `bevy_ui_navigation` is
//! a bit rough:
//! * You need to listen for [`NavEvent`],
//! * filter for the ones you care about (typically [`NavEvent::NoChanges`]),
//! * check what [`NavRequest`] triggered it,
//! * retrieve the focused entity from the event,
//! * check against your own queries what entity it is,
//! * write code for each case you want to handle
//!
//! It is not _awful_, but it requires a deep familiarity with this crates'
//! way of doing things:
//! ```rust
//! use bevy::prelude::*;
//! use bevy_ui_navigation::events::{NavEvent, NavRequest};
//!
//! #[derive(Component)]
//! enum MainMenuButton { Start, Options, Exit }
//! /// Marker component
//! #[derive(Component)] struct ActiveButton;
//!
//! fn handle_ui(
//!   mut events: EventReader<NavEvent>,
//!   buttons: Query<&MainMenuButton, With<ActiveButton>>,
//! ) {
//!   // iterate NavEvents
//!   for event in events.iter() {
//!     // Check for a `NoChanges` event with Action
//!     if let NavEvent::NoChanges { from, request: NavRequest::Action } = event {
//!       // Get the focused entity (from.first()), check the button for it.
//!       match buttons.get(*from.first()) {
//!         // Do things when specific button is activated
//!         Ok(MainMenuButton::Start) => {}
//!         Ok(MainMenuButton::Options) => {}
//!         Ok(MainMenuButton::Exit) => {}
//!         Err(_) => {}
//!       }
//!     }
//!   }
//! }
//! ```
//!
//! This module defines two [`SystemParam`]:
//! * [`NavEventQuery`]: A wrapper around a query to limit iteration to
//!   item activated this cycle.
//! * [`NavEventReader`]: A wrapper around `EventReader<NavEvent>` that
//!   add extra methods specific to iterating `NavEvent`s.
//!
//! Those `SystemParam` are accessed by specifying them as system arguments.
//!
//! ## [`NavEventReader`]
//!
//! [`NavEventReader`] works exactly like `EventReader` (you can even access
//! the inner reader) but gives you choice on how to iterate the `NavEvent`s.
//!
//! Check the [`NavEventReader`] docs for usage examples and use cases.
//!
//! ## [`NavEventQuery`]
//!
//! [`NavEventQuery`] works exactly like a query, appart that it doesn't define
//! `iter()`, but [`iter_requested()`](NavEventQuery::iter_requested) and
//! [`iter_activated()`](NavEventQuery::iter_activated). You use it exactly
//! like the bevy `Query` system parameter. This let you just iterate over
//! the entities that got activated since last frame.
//!
//! Check the [`NavEventQuery`] docs for usage examples and use cases.
//!
//! # Examples
//!
//! Those `SystemParam` should let you react to `NavEvent`s under your own terms.
//! The simplest way to handle `NavEvent`s is with the `NavEventQuery` parameter:
//!
//! ```rust
//! use bevy::prelude::*;
//! use bevy_ui_navigation::event_helpers::NavEventQuery;
//!
//! #[derive(Component)]
//! enum MainMenuButton { Start, Options, Exit }
//! /// Marker component
//! #[derive(Component)] struct ActiveButton;
//!
//! fn handle_ui(mut button_events: NavEventQuery<&MainMenuButton, With<ActiveButton>>) {
//!   // NOTE: this will silently ignore multiple navigation event at the same frame.
//!   // It should be a very rare occurance.
//!   match button_events.single_activated().ignore_remaining() {
//!     // Do things when specific button is activated
//!     Some(MainMenuButton::Start) => {}
//!     Some(MainMenuButton::Options) => {}
//!     Some(MainMenuButton::Exit) => {}
//!     None => {}
//!   }
//! }
//! ```
//!
//! # Disclaimer
//!
//! This is a very new API and it is likely to be too awkward to use in a real-world
//! use case. But the aim is to make it possible to use this API in 90% of cases. I
//! don't think it's currently the case. Feel free to open an issue discussing possible
//! improvements.
use std::ops::{Deref, DerefMut};

use bevy_ecs::{
    entity::Entity,
    event::EventReader,
    query::{QueryItem, ROQueryItem, WorldQuery},
    system::{Query, SystemParam},
};

use crate::events::{NavEvent, NavRequest};

/// A thing that should exist or not, but possibly could erroneously be multiple.
pub enum SingleResult<T> {
    /// There is exactly one `T`.
    One(T),
    /// There is exactly no `T`.
    None,
    /// There is more than one `T`, holding the first `T`.
    MoreThanOne(T),
}
impl<T> SingleResult<T> {
    /// Assume the [`SingleResult::MoreThanOne`] case doesn't exist.
    ///
    /// # Panics
    ///
    /// When `self` is [`SingleResult::MoreThanOne`].
    pub fn unwrap_opt(self) -> Option<T> {
        match self {
            Self::MoreThanOne(_) => panic!("There was more than one `SingleResult`"),
            Self::One(t) => Some(t),
            Self::None => None,
        }
    }
    /// Return contained value, not caring even if there is more than one result.
    pub fn ignore_remaining(self) -> Option<T> {
        match self {
            Self::MoreThanOne(t) | Self::One(t) => Some(t),
            Self::None => None,
        }
    }
    pub fn deref_mut(&mut self) -> SingleResult<&mut <T as Deref>::Target>
    where
        T: DerefMut,
    {
        match self {
            Self::MoreThanOne(t) => SingleResult::MoreThanOne(t.deref_mut()),
            Self::One(t) => SingleResult::One(t.deref_mut()),
            Self::None => SingleResult::None,
        }
    }
    fn new(from: Option<T>, is_multiple: bool) -> Self {
        match (from, is_multiple) {
            (Some(t), false) => Self::One(t),
            (Some(t), true) => Self::MoreThanOne(t),
            (None, _) => Self::None,
        }
    }
}

/// Types of [`NavEvent`] that can be "emitted" by focused elements.
pub enum NavEventType {
    /// [`NavEvent::Locked`].
    Locked,
    /// [`NavEvent::Unlocked`].
    Unlocked,
    /// [`NavEvent::FocusChanged`].
    FocusChanged,
    /// [`NavEvent::NoChanges`].
    NoChanges(NavRequest),
}

/// An [`EventReader<NavEvent>`] with methods to filter for meaningful events.
/// Use this like an `EventReader`, but with extra functionalities:
/// ```rust
/// # use bevy::prelude::*;
/// use bevy_ui_navigation::event_helpers::{NavEventReader, NavEventType};
/// use bevy_ui_navigation::events::NavRequest;
///
/// # #[derive(Component)] enum MainMenuButton { Start, Options, Exit }
/// # #[derive(Component)] struct ActiveButton;
/// fn handle_ui(
///     mut events: NavEventReader,
///     buttons: Query<&MainMenuButton, With<ActiveButton>>,
/// ) {
///   for (event_type, from) in events.type_iter() {
///     match (buttons.get(from), event_type) {
///       (Ok(MainMenuButton::Start), NavEventType::NoChanges(NavRequest::Action))  => {}
///       (Ok(MainMenuButton::Start), NavEventType::Locked) => {}
///       (Ok(MainMenuButton::Start), _) => {}
///       (Ok(MainMenuButton::Options), _) => {}
///       _ => {}
///       // etc..
///     }
///   }
/// }
/// ```
/// See methods documentation for what type of iterators you can get.
#[derive(SystemParam)]
pub struct NavEventReader<'w, 's> {
    pub events: EventReader<'w, 's, NavEvent>,
}
impl<'w, 's> NavEventReader<'w, 's> {
    /// Event reader of [`events`](NavEventReader::events) filtered
    /// to only keep [`NavEvent::NoChanges`].
    ///
    /// Note that this iterator usually has 0 or 1 element.
    pub fn unchanged(&mut self) -> impl DoubleEndedIterator<Item = (NavRequest, Entity)> + '_ {
        self.events.iter().filter_map(|event| {
            if let NavEvent::NoChanges { from, request } = event {
                Some((*request, *from.first()))
            } else {
                None
            }
        })
    }

    /// Iterate [`NavEvent`] by type.
    ///
    /// Note that this iterator usually has 0 or 1 element.
    pub fn type_iter(&mut self) -> impl DoubleEndedIterator<Item = (NavEventType, Entity)> + '_ {
        use NavEventType::{FocusChanged, Locked, NoChanges, Unlocked};
        self.events.iter().filter_map(|event| match event {
            NavEvent::FocusChanged { from, .. } => Some((FocusChanged, *from.first())),
            NavEvent::Locked(from) => Some((Locked, *from)),
            NavEvent::Unlocked(from) => Some((Unlocked, *from)),
            NavEvent::NoChanges { from, request } => Some((NoChanges(*request), *from.first())),
            _ => None,
        })
    }

    /// The entities that got an unhandled `request` [`NavRequest`] this frame.
    ///
    /// Note that this iterator usually has 0 or 1 element.
    pub fn caught(&mut self, request: NavRequest) -> impl DoubleEndedIterator<Item = Entity> + '_ {
        self.unchanged()
            .filter_map(move |(req, entity)| (req == request).then(|| entity))
    }

    /// The entities that got an unhandled [`NavRequest::Action`] this frame.
    ///
    /// An unhandled `Action` happens typically when the user presses the action
    /// key on a focused entity. The [`Entity`] returned here will be the focused
    /// entity.
    ///
    /// Typically there will be 0 or 1 such entity. It's technically possible to
    /// have more than one activated entity in the same frame, but extremely
    /// unlikely. Please account for that case.
    pub fn activated(&mut self) -> impl DoubleEndedIterator<Item = Entity> + '_ {
        self.caught(NavRequest::Action)
    }

    /// Variation of [`NavEventReader::caught`] where more than one `NavEvent` is
    /// explicitly excluded.
    pub fn single_caught(&mut self, request: NavRequest) -> SingleResult<Entity> {
        let mut iterated = self.caught(request);
        let first = iterated.next();
        let one_more = iterated.next().is_some();
        SingleResult::new(first, one_more)
    }

    /// Variation of [`NavEventReader::activated`] where more than one `NavEvent` is
    /// explicitly excluded.
    pub fn single_activated(&mut self) -> SingleResult<Entity> {
        self.single_caught(NavRequest::Action)
    }
}

/// Convinient wrapper around a query for a quick way of handling UI events.
///
/// See [the module level doc](crate::event_helpers) for justification and
/// use case, see the following methods documentation for specifics on how
/// to use this [`SystemParam`].
///
/// This is a bevy [`SystemParam`], you access it by specifying it as parameter
/// to your systems:
///
/// ```rust
/// # use bevy::prelude::*;
/// use bevy_ui_navigation::event_helpers::NavEventQuery;
/// # #[derive(Component)] enum MenuButton { StartGame, Quit, JoinFriend }
///
/// fn handle_ui(mut button_events: NavEventQuery<&MenuButton>) {}
/// ```
#[derive(SystemParam)]
pub struct NavEventQuery<'w, 's, Q: WorldQuery + 'static, F: 'static + WorldQuery = ()> {
    query: Query<'w, 's, Q, F>,
    events: NavEventReader<'w, 's>,
}
impl<'w, 's, Q: WorldQuery, F: WorldQuery> NavEventQuery<'w, 's, Q, F> {
    /// Like [`NavEventQuery::iter_activated`], but for mutable queries.
    ///
    /// You can use this method to get mutable access to items from the `Q`
    /// `WorldQuery`.
    ///
    /// It is however recommended that you use a [`NavEventReader`] method
    /// with your own queries if you have any kind of complex access pattern
    /// to your mutable data.
    ///
    /// With the `MenuButton` example from [`NavEventQuery::iter_activated`],
    /// you could use this method as follow:
    ///
    /// ```rust
    /// use bevy::prelude::*;
    /// use bevy::app::AppExit;
    /// use bevy_ui_navigation::event_helpers::NavEventQuery;
    /// # use bevy::ecs::system::SystemParam;
    /// # #[derive(SystemParam)] struct StartGameQuery<'w, 's> { foo: Query<'w, 's, ()> }
    /// #[derive(Component)]
    /// enum MenuButton {
    ///     StartGame,
    ///     Quit,
    ///     Counter(i64),
    /// }
    ///
    /// fn start_game(queries: &mut StartGameQuery) { /* ... */ }
    ///
    /// fn handle_menu_button(
    ///     mut buttons: NavEventQuery<&mut MenuButton>,
    ///     mut app_evs: EventWriter<AppExit>,
    ///     mut start_game_query: StartGameQuery,
    /// ) {
    ///     // WARNING: using `deref_mut` here triggers change detection regardless
    ///     // of whether we changed anything, but there is no other ways to
    ///     // pattern-match on `MenuButton` in rust in this case.
    ///     match buttons.single_activated_mut().deref_mut().unwrap_opt() {
    ///         Some(MenuButton::Counter(i)) => *i += 1,
    ///         Some(MenuButton::StartGame) => start_game(&mut start_game_query),
    ///         Some(MenuButton::Quit) => app_evs.send(AppExit),
    ///         None => {},
    ///     }
    /// }
    /// ```
    pub fn single_activated_mut(&mut self) -> SingleResult<QueryItem<Q>> {
        self.single_caught_mut(NavRequest::Action)
    }
    /// Like [`NavEventQuery::single_activated_mut`] but for non-mutable queries.
    ///
    /// ```rust
    /// use bevy::prelude::*;
    /// use bevy::app::AppExit;
    /// use bevy_ui_navigation::event_helpers::NavEventQuery;
    /// # use bevy::ecs::system::SystemParam;
    /// # #[derive(SystemParam)] struct StartGameQuery<'w, 's> { foo: Query<'w, 's, ()> }
    /// #[derive(Component)]
    /// enum MenuButton {
    ///     StartGame,
    ///     Quit,
    ///     Options,
    /// }
    ///
    /// fn start_game(queries: &mut StartGameQuery) { /* ... */ }
    ///
    /// fn handle_menu_button(
    ///     mut buttons: NavEventQuery<&MenuButton>,
    ///     mut app_evs: EventWriter<AppExit>,
    ///     mut start_game_query: StartGameQuery,
    /// ) {
    ///     match buttons.single_activated().unwrap_opt() {
    ///         Some(MenuButton::Options) => {/* do something optiony */},
    ///         Some(MenuButton::StartGame) => start_game(&mut start_game_query),
    ///         Some(MenuButton::Quit) => app_evs.send(AppExit),
    ///         None => {},
    ///     }
    /// }
    /// ```
    pub fn single_activated(&mut self) -> SingleResult<ROQueryItem<Q>> {
        match self.events.single_caught(NavRequest::Action) {
            SingleResult::MoreThanOne(e) => SingleResult::new(self.query.get(e).ok(), true),
            SingleResult::One(e) => SingleResult::new(self.query.get(e).ok(), false),
            SingleResult::None => SingleResult::None,
        }
    }

    /// Like [`NavEventQuery::single_activated_mut`] but for arbitrary
    /// `react_to` [`NavRequest`].
    pub fn single_caught_mut(&mut self, react_to: NavRequest) -> SingleResult<QueryItem<Q>> {
        match self.events.single_caught(react_to) {
            SingleResult::MoreThanOne(e) => SingleResult::new(self.query.get_mut(e).ok(), true),
            SingleResult::One(e) => SingleResult::new(self.query.get_mut(e).ok(), false),
            SingleResult::None => SingleResult::None,
        }
    }

    /// Iterate over items of `Q` that received a `react_to` [`NavRequest`]
    /// not handled by the navigation engine.
    ///
    /// This method is used like [`NavEventQuery::iter_activated`], but with
    /// the additional `react_to` argument.
    ///
    /// Note that this iterator is usualy of length 0 or 1.
    ///
    /// See the [`NavEventQuery::iter_activated`] documentation for details.
    pub fn iter_requested(&mut self, react_to: NavRequest) -> impl Iterator<Item = ROQueryItem<Q>> {
        let Self { events, query } = self;
        events.caught(react_to).filter_map(|e| query.get(e).ok())
    }

    /// Iterate over received [`NavEvent`] types associated with the
    /// corresponding result from `Q`.
    ///
    /// Note that this iterator is usualy of length 0 or 1.
    ///
    /// This is similar to [`NavEventReader::type_iter`], but instead of
    /// returning an entity, it returns the query item of `Q`
    pub fn iter_types(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = (NavEventType, ROQueryItem<Q>)> {
        let Self { events, query } = self;
        events
            .type_iter()
            .filter_map(|(t, e)| query.get(e).ok().map(|e| (t, e)))
    }

    /// Iterate over items of `Q` that received a [`NavRequest::Action`]
    /// not handled by the navigation engine.
    ///
    /// This method is very useful for UI logic. You typically want to react
    /// to [`NavEvent::NoChanges`] where the `request` field is
    /// [`NavRequest::Action`]. This happens when the user clicks a button or
    /// when they press the `Action` button on their controller.
    ///
    /// Note that this iterator is usualy of length 0 or 1.
    ///
    /// # Mutable queries
    ///
    /// This method cannot be used with `WorldQueries` with mutable
    /// world access. You should use [`NavEventQuery::single_activated_mut`] instead.
    ///
    /// # Example
    ///
    /// We have a menu where each button has a `MenuButton` component. We want
    /// to do something special when the player clicks a specific button, to
    /// react to specific buttons we would create a system that accepts a
    /// `NavEventQuery<&MenuButton>` and uses it as follow:
    ///
    /// ```rust
    /// use bevy::prelude::*;
    /// use bevy::app::AppExit;
    /// # use bevy::ecs::system::SystemParam;
    /// # #[derive(SystemParam)] struct StartGameQuery<'w, 's> { foo: Query<'w, 's, ()> }
    /// # #[derive(SystemParam)] struct JoinFriendQuery<'w, 's> { foo: Query<'w, 's, ()> }
    /// use bevy_ui_navigation::event_helpers::NavEventQuery;
    ///
    /// #[derive(Component)]
    /// enum MenuButton {
    ///     StartGame,
    ///     Quit,
    ///     JoinFriend,
    /// }
    ///
    /// fn start_game(queries: &mut StartGameQuery) { /* ... */ }
    /// fn join_friend(queries: &mut JoinFriendQuery) { /* ... */ }
    ///
    /// fn handle_menu_button(
    ///     mut buttons: NavEventQuery<&MenuButton>,
    ///     mut app_evs: EventWriter<AppExit>,
    ///     mut start_game_query: StartGameQuery,
    ///     mut join_friend_query: JoinFriendQuery,
    /// ) {
    ///     for activated_button in buttons.iter_activated() {
    ///         match activated_button {
    ///             MenuButton::StartGame => start_game(&mut start_game_query),
    ///             MenuButton::Quit => app_evs.send(AppExit),
    ///             MenuButton::JoinFriend => join_friend(&mut join_friend_query),
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// If you are curious how `StartGameQuery` was defined, check out the bevy
    /// [`SystemParam`] trait!
    pub fn iter_activated(&mut self) -> impl Iterator<Item = ROQueryItem<Q>> {
        self.iter_requested(NavRequest::Action)
    }
}
