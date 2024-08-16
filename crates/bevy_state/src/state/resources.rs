use std::ops::Deref;

use bevy_ecs::{
    change_detection::DetectChangesMut,
    system::{ResMut, Resource},
    world::{FromWorld, World},
};

use super::{freely_mutable_state::FreelyMutableState, states::States};

#[cfg(feature = "bevy_reflect")]
use bevy_ecs::prelude::ReflectResource;

/// A finite-state machine whose transitions have associated schedules
/// ([`OnEnter(state)`](crate::state::OnEnter) and [`OnExit(state)`](crate::state::OnExit)).
///
/// The current state value can be accessed through this resource. To *change* the state,
/// queue a transition in the [`NextState<S>`] resource, and it will be applied during the
/// [`StateTransition`](crate::transition::StateTransition) schedule - which by default runs after `PreUpdate`.
///
/// You can also manually trigger the [`StateTransition`](crate::transition::StateTransition) schedule to apply the changes
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
    reflect(Resource)
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

/// The next state of [`State<S>`].
///
/// This can be fetched as a resource and used to queue state transitions.
/// To queue a transition, call [`NextState::set`] or mutate the value to [`NextState::Pending`] directly.
///
/// Note that these transitions can be overridden by other systems:
/// only the actual value of this resource during the [`StateTransition`](crate::transition::StateTransition) schedule matters.
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
    reflect(Resource)
)]
pub enum NextState<S: FreelyMutableState> {
    /// No state transition is pending
    #[default]
    Unchanged,
    /// There is a pending transition for state `S`
    Pending(S),
}

impl<S: FreelyMutableState> NextState<S> {
    /// Tentatively set a pending state transition to `Some(state)`.
    pub fn set(&mut self, state: S) {
        *self = Self::Pending(state);
    }

    /// Remove any pending changes to [`State<S>`]
    pub fn reset(&mut self) {
        *self = Self::Unchanged;
    }
}

pub(crate) fn take_next_state<S: FreelyMutableState>(
    next_state: Option<ResMut<NextState<S>>>,
) -> Option<S> {
    let mut next_state = next_state?;

    match std::mem::take(next_state.bypass_change_detection()) {
        NextState::Pending(x) => {
            next_state.set_changed();
            Some(x)
        }
        NextState::Unchanged => None,
    }
}
