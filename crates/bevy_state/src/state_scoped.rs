#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    entity_disabling::Disabled,
    message::MessageReader,
    query::{Allow, AnyOf},
    system::{Commands, Query},
};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

use crate::state::{StateTransitionEvent, States};

/// Entities marked with this component will be despawned
/// upon exiting the given state.
///
/// If you need to disable this behavior, add the attribute `#[states(scoped_entities = false)]` when deriving [`States`].
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

/// Entities marked with this component will be despawned
/// upon exiting the state that matches the predicate.
///
/// If you need to disable this behavior, add the attribute `#[states(scoped_entities = false)]` when deriving [`States`].
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
///     Level(u8),
/// }
///
/// # #[derive(Component)]
/// # struct Player;
///
/// fn spawn_player(mut commands: Commands) {
///     commands.spawn((
///         DespawnOnExitWith(|entered_state| matches!(entered_state, GameState::Level(2))),
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
/// app.add_systems(OnEnter(GameState::Level(2)), spawn_player);
/// ```
#[derive(Component, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component, Clone))]
pub struct DespawnOnExitWith<S: States>(pub fn(&S) -> bool);

impl<S> Default for DespawnOnExitWith<S>
where
    S: States + Default,
{
    fn default() -> Self {
        Self(|state| *state == S::default())
    }
}

/// Despawns entities marked with [`DespawnOnExit<S>`] when their state no
/// longer matches the world state.
///
/// If the entity has already been despawned no warning will be emitted.
pub fn despawn_entities_on_exit_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, AnyOf<(&DespawnOnExit<S>, &DespawnOnExitWith<S>)>), Allow<Disabled>>,
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
    for (entity, (exit, exit_with)) in &query {
        if exit_with.is_some_and(|exit_with| exit_with.0(exited)) {
            commands.entity(entity).try_despawn();
        }
        if exit.is_some_and(|exit| exit.0 == *exited) {
            commands.entity(entity).try_despawn();
        }
    }
}

/// Entities marked with this component will be despawned
/// upon entering the given state.
///
/// If you need to disable this behavior, add the attribute `#[states(scoped_entities = false)]` when deriving [`States`].
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

impl<S: States + Default> Default for DespawnOnEnter<S> {
    fn default() -> Self {
        Self(S::default())
    }
}

/// Entities marked with this component will be despawned
/// upon entering the state that matches the predicate.
///
/// If you need to disable this behavior, add the attribute `#[states(scoped_entities = false)]` when deriving [`States`].
///
/// ```
/// use bevy_state::prelude::*;
/// use bevy_ecs::{prelude::*, system::ScheduleSystem};
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     GameOver,
///     SettingsMenu,
///     InGame,
///     Level(u8),
/// }
///
/// # #[derive(Component)]
/// # struct Player;
///
/// fn spawn_player(mut commands: Commands) {
///     commands.spawn((
///         DespawnOnEnterWith(|entered_state| {
///             matches!(entered_state, GameState::MainMenu | GameState::GameOver)
///         }),
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
pub struct DespawnOnEnterWith<S>(pub fn(&S) -> bool);

impl<S: States + Default> Default for DespawnOnEnterWith<S> {
    fn default() -> Self {
        Self(|state| *state == S::default())
    }
}

/// Despawns entities marked with [`DespawnOnEnter<S>`], or [`DespawnOnEnterWith<S>`] when their state
/// matches the world state.
///
/// If the entity has already been despawned no warning will be emitted.
pub fn despawn_entities_on_enter_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, AnyOf<(&DespawnOnEnter<S>, &DespawnOnEnterWith<S>)>), Allow<Disabled>>,
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
    for (entity, (enter, enter_with)) in &query {
        if enter_with.is_some_and(|enter_with| enter_with.0(entered)) {
            commands.entity(entity).try_despawn();
        }
        if enter.is_some_and(|enter| enter.0 == *entered) {
            commands.entity(entity).try_despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bevy_app::App;

    use crate::{
        app::{AppExtStates, StatesPlugin},
        prelude::CommandsStatesExt,
    };

    #[test]
    fn despawn_on_exit_from_computed_state() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
        enum State {
            On,
            Off,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        struct ComputedState;
        impl bevy_state::state::ComputedStates for ComputedState {
            type SourceStates = State;

            fn compute(sources: Self::SourceStates) -> Option<Self> {
                match sources {
                    State::On => Some(ComputedState),
                    State::Off => None,
                }
            }
        }

        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        app.insert_state(State::On);
        app.add_computed_state::<ComputedState>();
        app.update();

        assert_eq!(
            app.world()
                .resource::<bevy_state::state::State<State>>()
                .get(),
            &State::On
        );
        assert_eq!(
            app.world()
                .resource::<bevy_state::state::State<ComputedState>>()
                .get(),
            &ComputedState
        );

        let entity = app.world_mut().spawn(DespawnOnExit(ComputedState)).id();
        assert!(app.world().get_entity(entity).is_ok());

        app.world_mut().commands().set_state(State::Off);
        app.update();

        assert_eq!(
            app.world()
                .resource::<bevy_state::state::State<State>>()
                .get(),
            &State::Off
        );
        assert!(app
            .world()
            .get_resource::<bevy_state::state::State<ComputedState>>()
            .is_none());
        assert!(app.world().get_entity(entity).is_err());
    }
}
