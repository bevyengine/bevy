use std::fmt::Debug;

use std::hash::Hash;

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
/// use bevy_state::prelude::*;
/// use bevy_ecs::prelude::IntoSystemConfigs;
/// use bevy_ecs::system::ResMut;
///
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     SettingsMenu,
///     InGame,
/// }
///
/// fn handle_escape_pressed(mut next_state: ResMut<NextState<GameState>>) {
/// #   let escape_pressed = true;
///     if escape_pressed {
///         next_state.set(GameState::SettingsMenu);
///     }
/// }
///
/// fn open_settings_menu() {
///     // Show the settings menu...
/// }
///
/// # struct AppMock;
/// # impl AppMock {
/// #     fn add_systems<S, M>(&mut self, schedule: S, systems: impl IntoSystemConfigs<M>) {}
/// # }
/// # struct Update;
/// # let mut app = AppMock;
///
/// app.add_systems(Update, handle_escape_pressed.run_if(in_state(GameState::MainMenu)));
/// app.add_systems(OnEnter(GameState::SettingsMenu), open_settings_menu);
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` can not be used as a state",
    label = "invalid state",
    note = "consider annotating `{Self}` with `#[derive(States)]`"
)]
pub trait States: 'static + Send + Sync + Clone + PartialEq + Eq + Hash + Debug {
    /// How many other states this state depends on.
    /// Used to help order transitions and de-duplicate [`ComputedStates`](crate::state::ComputedStates), as well as prevent cyclical
    /// `ComputedState` dependencies.
    const DEPENDENCY_DEPTH: usize = 1;
}
