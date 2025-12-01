use core::ops::Deref;

use bevy_ecs::{
    change_detection::DetectChangesMut,
    resource::Resource,
    system::ResMut,
    world::{FromWorld, World},
};

use super::{freely_mutable_state::FreelyMutableState, states::States};

#[cfg(feature = "bevy_reflect")]
use bevy_ecs::prelude::ReflectResource;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::ReflectDefault;

/// A finite-state machine whose transitions have associated schedules
/// ([`OnEnter(state)`](crate::state::OnEnter) and [`OnExit(state)`](crate::state::OnExit)).
///
/// The current state value can be accessed through this resource. To *change* the state,
/// queue a transition in the [`NextState<S>`] resource, and it will be applied during the
/// [`StateTransition`](crate::state::StateTransition) schedule - which by default runs after `PreUpdate`.
///
/// You can also manually trigger the [`StateTransition`](crate::state::StateTransition) schedule to apply the changes
/// at an arbitrary time.
///
/// The starting state is defined via the [`Default`] implementation for `S`.
///
/// ```
/// use bevy_state::prelude::*;
/// use bevy_ecs::prelude::*;
/// use bevy_state_macros::States;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     SettingsMenu,
///     InGame,
/// }
///
/// fn game_logic(game_state: Res<State<GameState>>) {
///     match game_state.get() {
///         GameState::InGame => {
///             // Run game logic here...
///         },
///         _ => {},
///     }
/// }
/// ```
#[derive(Resource, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource, Debug, PartialEq)
)]
pub struct State<S: States>(pub(crate) S);

impl<S: States> State<S> {
    /// Creates a new state with a specific value.
    ///
    /// To change the state use [`NextState<S>`] rather than using this to modify the `State<S>`.
    pub fn new(state: S) -> Self {
        Self(state)
    }

    /// Get the current state.
    pub fn get(&self) -> &S {
        &self.0
    }
}

impl<S: States + FromWorld> FromWorld for State<S> {
    fn from_world(world: &mut World) -> Self {
        Self(S::from_world(world))
    }
}

impl<S: States> PartialEq<S> for State<S> {
    fn eq(&self, other: &S) -> bool {
        self.get() == other
    }
}

impl<S: States> Deref for State<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

/// The previous state of [`State<S>`].
///
/// This resource holds the state value that was active immediately **before** the
/// most recent state transition. It is primarily useful for logic that runs
/// during state exit or transition schedules ([`OnExit`](crate::state::OnExit), [`OnTransition`](crate::state::OnTransition)).
///
/// It is inserted into the world only after the first state transition occurs. It will
/// remain present even if the primary state is removed (e.g., when a
/// [`SubStates`](crate::state::SubStates) or [`ComputedStates`](crate::state::ComputedStates) instance ceases to exist).
///
/// Use `Option<Res<PreviousState<S>>>` to access it, as it will not exist
/// before the first transition.
///
/// ```
/// use bevy_state::prelude::*;
/// use bevy_ecs::prelude::*;
/// use bevy_state_macros::States;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     InGame,
/// }
///
/// // This system might run in an OnExit schedule
/// fn log_previous_state(previous_state: Option<Res<PreviousState<GameState>>>) {
///     if let Some(previous) = previous_state {
///         // If this system is in OnExit(InGame), the previous state is what we
///         // were in before InGame.
///         println!("Transitioned from: {:?}", previous.get());
///     }
/// }
/// ```
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource, Debug, PartialEq)
)]
pub struct PreviousState<S: States>(pub(crate) S);

impl<S: States> PreviousState<S> {
    /// Get the previous state.
    pub fn get(&self) -> &S {
        &self.0
    }
}

impl<S: States> Deref for PreviousState<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The next state of [`State<S>`].
///
/// This can be fetched as a resource and used to queue state transitions.
/// To queue a transition, call [`NextState::set`] or mutate the value to [`NextState::Pending`] directly.
///
/// Note that these transitions can be overridden by other systems:
/// only the actual value of this resource during the [`StateTransition`](crate::state::StateTransition) schedule matters.
///
/// ```
/// use bevy_state::prelude::*;
/// use bevy_ecs::prelude::*;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     SettingsMenu,
///     InGame,
/// }
///
/// fn start_game(mut next_game_state: ResMut<NextState<GameState>>) {
///     next_game_state.set(GameState::InGame);
/// }
/// ```
#[derive(Resource, Debug, Default, Clone)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource, Default, Debug)
)]
pub enum NextState<S: FreelyMutableState> {
    /// No state transition is pending
    #[default]
    Unchanged,
    /// There is a pending transition for state `S`
    Pending(S),
    /// There is a pending transition for state `S`
    ///
    /// This will not trigger state transitions schedules if the target state is the same as the current one.
    PendingIfNeq(S),
}

impl<S: FreelyMutableState> NextState<S> {
    /// Tentatively set a pending state transition to `Some(state)`.
    ///
    /// This will run the state transition schedules [`OnEnter`](crate::state::OnEnter) and [`OnExit`](crate::state::OnExit).
    /// If you want to skip those schedules for the same where we are transitioning to the same state, use [`set_if_neq`](Self::set_if_neq) instead.
    pub fn set(&mut self, state: S) {
        *self = Self::Pending(state);
    }

    /// Tentatively set a pending state transition to `Some(state)`.
    ///
    /// Like [`set`](Self::set), but will not run any state transition schedules if the target state is the same as the current one.
    /// If [`set`](Self::set) has already been called in the same frame with the same state, the transition schedules will be run anyways.
    pub fn set_if_neq(&mut self, state: S) {
        if !matches!(self, Self::Pending(s) if s == &state) {
            *self = Self::PendingIfNeq(state);
        }
    }

    /// Remove any pending changes to [`State<S>`]
    pub fn reset(&mut self) {
        *self = Self::Unchanged;
    }
}

pub(crate) fn take_next_state<S: FreelyMutableState>(
    next_state: Option<ResMut<NextState<S>>>,
) -> Option<(S, bool)> {
    let mut next_state = next_state?;

    match core::mem::take(next_state.bypass_change_detection()) {
        NextState::Pending(x) => {
            next_state.set_changed();
            Some((x, true))
        }
        NextState::PendingIfNeq(x) => {
            next_state.set_changed();
            Some((x, false))
        }
        NextState::Unchanged => None,
    }
}
