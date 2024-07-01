use bevy_ecs::schedule::Schedule;

use super::{freely_mutable_state::FreelyMutableState, state_set::StateSet, states::States};
pub use bevy_state_macros::SubStates;

/// A sub-state is a state that exists only when the source state meet certain conditions,
/// but unlike [`ComputedStates`](crate::state::ComputedStates) - while they exist they can be manually modified.
///
/// The default approach to creating [`SubStates`] is using the derive macro, and defining a single source state
/// and value to determine it's existence.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_state::prelude::*;
///
/// #[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// enum AppState {
///     #[default]
///     Menu,
///     InGame
/// }
///
///
/// #[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// #[source(AppState = AppState::InGame)]
/// enum GamePhase {
///     #[default]
///     Setup,
///     Battle,
///     Conclusion
/// }
/// ```
///
/// you can then add it to an App, and from there you use the state as normal:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_state::prelude::*;
///
/// # struct App;
/// # impl App {
/// #   fn new() -> Self { App }
/// #   fn init_state<S>(&mut self) -> &mut Self {self}
/// #   fn add_sub_state<S>(&mut self) -> &mut Self {self}
/// # }
/// # struct AppState;
/// # struct GamePhase;
///
///     App::new()
///         .init_state::<AppState>()
///         .add_sub_state::<GamePhase>();
/// ```
///
/// In more complex situations, the recommendation is to use an intermediary computed state, like so:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_state::prelude::*;
///
/// /// Computed States require some state to derive from
/// #[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// enum AppState {
///     #[default]
///     Menu,
///     InGame { paused: bool }
/// }
///
/// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// struct InGame;
///
/// impl ComputedStates for InGame {
///     /// We set the source state to be the state, or set of states,
///     /// we want to depend on. Any of the states can be wrapped in an Option.
///     type SourceStates = Option<AppState>;
///
///     /// We then define the compute function, which takes in the AppState
///     fn compute(sources: Option<AppState>) -> Option<Self> {
///         match sources {
///             /// When we are in game, we want to return the InGame state
///             Some(AppState::InGame { .. }) => Some(InGame),
///             /// Otherwise, we don't want the `State<InGame>` resource to exist,
///             /// so we return None.
///             _ => None
///         }
///     }
/// }
///
/// #[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// #[source(InGame = InGame)]
/// enum GamePhase {
///     #[default]
///     Setup,
///     Battle,
///     Conclusion
/// }
/// ```
///
/// However, you can also manually implement them. If you do so, you'll also need to manually implement the `States` & `FreelyMutableState` traits.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_state::prelude::*;
/// # use bevy_state::state::{FreelyMutableState, NextState};
///
/// /// Computed States require some state to derive from
/// #[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)]
/// enum AppState {
///     #[default]
///     Menu,
///     InGame { paused: bool }
/// }
///
/// #[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// enum GamePhase {
///     Setup,
///     Battle,
///     Conclusion
/// }
///
/// impl SubStates for GamePhase {
///     /// We set the source state to be the state, or set of states,
///     /// we want to depend on. Any of the states can be wrapped in an Option.
///     type SourceStates = Option<AppState>;
///
///     /// We then define the compute function, which takes in the [`Self::SourceStates`]
///     fn should_exist(sources: Option<AppState>) -> Option<Self> {
///         match sources {
///             /// When we are in game, we want a GamePhase state to exist.
///             /// We can set the initial value here or overwrite it through [`NextState`].
///             Some(AppState::InGame { .. }) => Some(Self::Setup),
///             /// If we don't want the `State<GamePhase>` resource to exist we return [`None`].
///             _ => None
///         }
///     }
/// }
///
/// impl States for GamePhase {
///     const DEPENDENCY_DEPTH : usize = <GamePhase as SubStates>::SourceStates::SET_DEPENDENCY_DEPTH + 1;
/// }
///
/// impl FreelyMutableState for GamePhase {}
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` can not be used as a sub-state",
    label = "invalid sub-state",
    note = "consider annotating `{Self}` with `#[derive(SubStates)]`"
)]
pub trait SubStates: States + FreelyMutableState {
    /// The set of states from which the [`Self`] is derived.
    ///
    /// This can either be a single type that implements [`States`], or a tuple
    /// containing multiple types that implement [`States`], or any combination of
    /// types implementing [`States`] and Options of types implementing [`States`].
    type SourceStates: StateSet;

    /// This function gets called whenever one of the [`SourceStates`](Self::SourceStates) changes.
    /// The result is used to determine the existence of [`State<Self>`](crate::state::State).
    ///
    /// If the result is [`None`], the [`State<Self>`](crate::state::State) resource will be removed from the world,
    /// otherwise if the [`State<Self>`](crate::state::State) resource doesn't exist
    /// it will be created from the returned [`Some`] as the initial state.
    ///
    /// Value within [`Some`] is ignored if the state already exists in the world
    /// and only symbolises that the state should still exist.
    ///
    /// Initial value can also be overwritten by [`NextState`](crate::state::NextState).
    fn should_exist(sources: Self::SourceStates) -> Option<Self>;

    /// This function sets up systems that compute the state whenever one of the [`SourceStates`](Self::SourceStates)
    /// change. It is called by `App::add_computed_state`, but can be called manually if `App` is not
    /// used.
    fn register_sub_state_systems(schedule: &mut Schedule) {
        Self::SourceStates::register_sub_state_systems_in_schedule::<Self>(schedule);
    }
}
