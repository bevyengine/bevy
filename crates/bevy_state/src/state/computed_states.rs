use bevy_ecs::schedule::Schedule;

use super::{StateSet, States};

/// A state can be modified by a computation function.
///
/// A computed state is a state that is deterministically derived from a set of source states,
/// via a function provided to [`add_state_computation`](crate::app::AppExtStates::add_state_computation).
/// The function is called whenever any state of the source [`StateSet`] changes,
/// and the result becomes the computed state's value.
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
/// #[derive(States, Clone, PartialEq, Eq, Hash, Debug)]
/// #[computed]
/// struct InGame;
///
/// /// We give as parameter the state, or a tuple of states, that we want to depend on.
/// /// You can also wrap each state in an Option, if you want the state computation to
/// /// execute even if the source state doesn't currently exist in the world.
/// fn compute_in_game(sources: AppState) -> Option<InGame> {
///     match sources {
///         /// When we are in game, we want to return the InGame state
///         AppState::InGame { .. } => Some(InGame),
///         /// Otherwise, we don't want the `State<InGame>` resource to exist,
///         /// so we return None.
///         _ => None
///     }
/// }
/// ```
///
/// you can then add it to an App, and from there you use the state as normal
///
/// ```
/// # use bevy_state::prelude::*;
/// # use bevy_ecs::prelude::*;
/// # struct App;
/// # impl App {
/// #   fn new() -> Self { App }
/// #   fn init_state<S>(&mut self) -> &mut Self {self}
/// #   fn add_state_computation(&mut self, a: fn()) -> &mut Self {self}
/// # }
/// # struct AppState;
/// # struct InGame;
/// # fn compute_in_game() {}
///
/// App::new()
///     .init_state::<AppState>()
///     .add_state_computation(compute_in_game);
/// ```
pub trait ComputedStates: States {}

/// This function sets up systems that compute the state whenever one of the [`SourceStates`](Self::SourceStates)
/// change. It is called by `App::add_computed_state`, but can be called manually if `App` is not
/// used.
pub fn register_computed_state_systems<T: ComputedStates, SourceStates: StateSet>(
    schedule: &mut Schedule,
    f: impl Fn(SourceStates) -> Option<T> + Send + Sync + 'static,
) {
    SourceStates::register_computed_state_systems_in_schedule(schedule, f);
}
