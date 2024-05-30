use std::fmt::Debug;
use std::hash::Hash;

use bevy_ecs::schedule::Schedule;

use super::state_set::StateSet;
use super::states::States;

/// A state whose value is automatically computed based on the values of other [`States`].
///
/// A **computed state** is a state that is deterministically derived from a set of `SourceStates`.
/// The [`StateSet`] is passed into the `compute` method whenever one of them changes, and the
/// result becomes the state's value.
///
/// ```
/// # use bevy_state::prelude::*;
/// # use bevy_ecs::prelude::*;
///
/// /// Computed States require some state to derive from
/// #[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// enum AppState {
///     #[default]
///     Menu,
///     InGame { paused: bool }
/// }
///
///
/// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// struct InGame;
///
/// impl ComputedStates for InGame {
///     /// We set the source state to be the state, or a tuple of states,
///     /// we want to depend on. You can also wrap each state in an Option,
///     /// if you want the computed state to execute even if the state doesn't
///     /// currently exist in the world.
///     type SourceStates = AppState;
///
///     /// We then define the compute function, which takes in
///     /// your SourceStates
///     fn compute(sources: AppState) -> Option<Self> {
///         match sources {
///             /// When we are in game, we want to return the InGame state
///             AppState::InGame { .. } => Some(InGame),
///             /// Otherwise, we don't want the `State<InGame>` resource to exist,
///             /// so we return None.
///             _ => None
///         }
///     }
/// }
/// ```
///
/// you can then add it to an App, and from there you use the state as normal
///
/// ```
/// # use bevy_state::prelude::*;
/// # use bevy_ecs::prelude::*;
///
/// # struct App;
/// # impl App {
/// #   fn new() -> Self { App }
/// #   fn init_state<S>(&mut self) -> &mut Self {self}
/// #   fn add_computed_state<S>(&mut self) -> &mut Self {self}
/// # }
/// # struct AppState;
/// # struct InGame;
///
///     App::new()
///         .init_state::<AppState>()
///         .add_computed_state::<InGame>();
/// ```
pub trait ComputedStates: 'static + Send + Sync + Clone + PartialEq + Eq + Hash + Debug {
    /// The set of states from which the [`Self`] is derived.
    ///
    /// This can either be a single type that implements [`States`], an Option of a type
    /// that implements [`States`], or a tuple
    /// containing multiple types that implement [`States`] or Optional versions of them.
    ///
    /// For example, `(MapState, EnemyState)` is valid, as is `(MapState, Option<EnemyState>)`
    type SourceStates: StateSet;

    /// Computes the next value of [`State<Self>`](crate::state::State).
    /// This function gets called whenever one of the [`SourceStates`](Self::SourceStates) changes.
    ///
    /// If the result is [`None`], the [`State<Self>`](crate::state::State) resource will be removed from the world.
    fn compute(sources: Self::SourceStates) -> Option<Self>;

    /// This function sets up systems that compute the state whenever one of the [`SourceStates`](Self::SourceStates)
    /// change. It is called by `App::add_computed_state`, but can be called manually if `App` is not
    /// used.
    fn register_computed_state_systems(schedule: &mut Schedule) {
        Self::SourceStates::register_computed_state_systems_in_schedule::<Self>(schedule);
    }
}

impl<S: ComputedStates> States for S {
    const DEPENDENCY_DEPTH: usize = S::SourceStates::SET_DEPENDENCY_DEPTH + 1;
}
