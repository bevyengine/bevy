use std::fmt::Debug;

use std::hash::Hash;

pub use bevy_state_macros::States;

/// Types that can define world-wide states in a finite-state machine.
///
/// The [`Default`] trait defines the starting state.
/// Multiple states can be defined for the same world,
/// allowing you to classify the state of the world across orthogonal dimensions.
/// You can access the current state of type `T` with the [`State<T>`](crate::state::State) resource,
/// and the queued state with the [`NextState<T>`](crate::state::NextState) resource.
///
/// State transitions typically occur in the [`OnEnter<T::Variant>`](crate::state::OnEnter) and [`OnExit<T::Variant>`](crate::state::OnExit) schedules,
/// which can be run by triggering the [`StateTransition`](crate::state::StateTransition) schedule.
///
/// Types used as [`ComputedStates`](crate::state::ComputedStates) do not need to and should not derive [`States`].
/// [`ComputedStates`](crate::state::ComputedStates) should not be manually mutated: functionality provided
/// by the [`States`] derive and the associated [`FreelyMutableState`](crate::state::FreelyMutableState) trait.
///
/// # Example
///
/// ```
/// use bevy_state::prelude::States;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///  #[default]
///   MainMenu,
///   SettingsMenu,
///   InGame,
/// }
///
/// ```
pub trait States: 'static + Send + Sync + Clone + PartialEq + Eq + Hash + Debug {
    /// How many other states this state depends on.
    /// Used to help order transitions and de-duplicate [`ComputedStates`](crate::state::ComputedStates), as well as prevent cyclical
    /// `ComputedState` dependencies.
    const DEPENDENCY_DEPTH: usize = 1;
}
