use std::{marker::PhantomData, ops::Deref};

use bevy_ecs::{
    system::Resource,
    world::{FromWorld, World},
};

use super::{
    computed_states::ComputedStates, freely_mutable_state::FreelyMutableState, states::States,
};

#[cfg(feature = "bevy_reflect")]
use bevy_ecs::prelude::ReflectResource;

/// A finite-state machine whose transitions have associated schedules
/// ([`OnEnter(state)`](crate::state::OnEnter) and [`OnExit(state)`](crate::state::OnExit)).
///
/// The current state value can be accessed through this resource. To *change* the state,
/// queue a transition in the [`NextState<S>`] resource, and it will be applied by the next
/// [`apply_state_transition::<S>`](crate::state::apply_state_transition) system.
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
/// To queue a transition, just set the contained value to `Some(next_state)`.
///
/// Note that these transitions can be overridden by other systems:
/// only the actual value of this resource at the time of [`apply_state_transition`](crate::state::apply_state_transition) matters.
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
#[derive(Resource, Debug, Default)]
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

/// The flag that determines whether the computed state `S` will be refreshed this frame.
///
/// Refreshing a state will apply its state transition even if nothing has changed, in which case
/// there will be a state transition from the current state to itself.
#[derive(Resource, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Resource)
)]
pub struct RefreshState<S: ComputedStates>(pub bool, PhantomData<S>);

impl<S: ComputedStates> Default for RefreshState<S> {
    fn default() -> Self {
        Self(false, PhantomData)
    }
}

impl<S: ComputedStates> RefreshState<S> {
    /// Plan to refresh the computed state `S`.
    pub fn refresh(&mut self) {
        self.0 = true;
    }
}
