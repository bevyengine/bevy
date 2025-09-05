use bevy_app::SubApp;
#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EventReader,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

use crate::state::{StateTransition, StateTransitionEvent, StateTransitionSystems, States};

/// Entities marked with this component will be removed
/// when the world's state of the matching type no longer matches the supplied value.
///
/// ```
/// use bevy_state::prelude::*;
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::system::ScheduleSystem;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     SettingsMenu,
///     InGame,
/// }
///
/// # #[derive(Component)]
/// # struct Player;
///
/// fn spawn_player(mut commands: Commands) {
///     commands.spawn((
///         DespawnOnExit(GameState::InGame),
///         Player
///     ));
/// }
///
/// # struct AppMock;
/// # impl AppMock {
/// #     fn init_state<S>(&mut self) {}
/// #     fn add_systems<S, M>(&mut self, schedule: S, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) {}
/// # }
/// # struct Update;
/// # let mut app = AppMock;
///
/// app.init_state::<GameState>();
/// app.add_systems(OnEnter(GameState::InGame), spawn_player);
/// ```
#[derive(Component, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component, Clone))]
pub struct DespawnOnExit<S: States>(pub S);

impl<S> Default for DespawnOnExit<S>
where
    S: States + Default,
{
    fn default() -> Self {
        Self(S::default())
    }
}

/// Despawns entities marked with [`DespawnOnExit<S>`] when their state no
/// longer matches the world state.
pub fn despawn_entities_on_exit_state<S: States>(
    mut commands: Commands,
    mut transitions: EventReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &DespawnOnExit<S>)>,
) {
    // We use the latest event, because state machine internals generate at most 1
    // transition event (per type) each frame. No event means no change happened
    // and we skip iterating all entities.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited {
        return;
    }
    let Some(exited) = &transition.exited else {
        return;
    };
    for (entity, binding) in &query {
        if binding.0 == *exited {
            commands.entity(entity).despawn();
        }
    }
}

/// Entities marked with this component will be despawned
/// upon entering the given state.
///
/// ```
/// use bevy_state::prelude::*;
/// use bevy_ecs::{prelude::*, system::ScheduleSystem};
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     SettingsMenu,
///     InGame,
/// }
///
/// # #[derive(Component)]
/// # struct Player;
///
/// fn spawn_player(mut commands: Commands) {
///     commands.spawn((
///         DespawnOnEnter(GameState::MainMenu),
///         Player
///     ));
/// }
///
/// # struct AppMock;
/// # impl AppMock {
/// #     fn init_state<S>(&mut self) {}
/// #     fn add_systems<S, M>(&mut self, schedule: S, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) {}
/// # }
/// # struct Update;
/// # let mut app = AppMock;
///
/// app.init_state::<GameState>();
/// app.add_systems(OnEnter(GameState::InGame), spawn_player);
/// ```
#[derive(Component, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct DespawnOnEnter<S: States>(pub S);

/// Despawns entities marked with [`DespawnOnEnter<S>`] when their state
/// no longer matches the world state.
pub fn despawn_entities_on_enter_state<S: States>(
    mut commands: Commands,
    mut transitions: EventReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &DespawnOnEnter<S>)>,
) {
    // We use the latest event, because state machine internals generate at most 1
    // transition event (per type) each frame. No event means no change happened
    // and we skip iterating all entities.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited {
        return;
    }
    let Some(entered) = &transition.entered else {
        return;
    };
    for (entity, binding) in &query {
        if binding.0 == *entered {
            commands.entity(entity).despawn();
        }
    }
}

/// Sets up state-scoped entities by adding the [`despawn_entities_on_exit_state`] and [`despawn_entities_on_enter_state`] systems.
///
/// Runs automatically when a State is installed using [`AppExtStates`](crate::app::AppExtStates).
#[doc(hidden)]
pub fn setup_state_scoped_entities<S: States>(app: &mut SubApp) {
    // Note: We work with `StateTransition` in set
    // `StateTransitionSystems::ExitSchedules` rather than `OnExit`, because
    // `OnExit` only runs for one specific variant of the state.
    app.add_systems(
        StateTransition,
        despawn_entities_on_exit_state::<S>.in_set(StateTransitionSystems::ExitSchedules),
    )
    // Note: We work with `StateTransition` in set
    // `StateTransitionSystems::EnterSchedules` rather than `OnEnter`, because
    // `OnEnter` only runs for one specific variant of the state.
    .add_systems(
        StateTransition,
        despawn_entities_on_enter_state::<S>.in_set(StateTransitionSystems::EnterSchedules),
    );
}
