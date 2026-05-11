use alloc::boxed::Box;

#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    entity_disabling::Disabled,
    hierarchy::Children,
    message::MessageReader,
    query::{Allow, With},
    system::{Commands, Query},
};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

use crate::state::{StateTransitionEvent, States};

/// Entities marked with this component will be despawned
/// when a [`StateTransitionEvent<S>`] matching the given predicate is sent.
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
///     GameOver,
/// }
///
/// # #[derive(Component)]
/// # struct Player;
///
/// fn spawn_player(mut commands: Commands) {
///     commands.spawn((
///         DespawnWhen::new(|transition| {
///             matches!(
///                 transition.entered,
///                 Some(GameState::MainMenu) | Some(GameState::GameOver)
///             )
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
///
/// See also [`DespawnOnExit`] and [`DespawnOnEnter`].
#[derive(Component)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct DespawnWhen<S: States> {
    /// The predicate that is ran when message [`StateTransitionEvent<S>`] is sent.
    pub state_transition_evaluator:
        Box<dyn Fn(&StateTransitionEvent<S>) -> bool + Sync + Send + 'static>,
}

impl<S: States> DespawnWhen<S> {
    /// Creates a [`DespawnWhen`] for the given predicate.
    pub fn new(f: impl Fn(&StateTransitionEvent<S>) -> bool + Sync + Send + 'static) -> Self {
        Self {
            state_transition_evaluator: Box::new(f),
        }
    }
}

/// Despawns entities marked with [`DespawnWhen<S>`] when the state transition message matches their
/// predicate.
///
/// If the entity has already been despawned no warning will be emitted.
pub fn despawn_entities_when_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &DespawnWhen<S>), Allow<Disabled>>,
) {
    // We use the latest event, because state machine internals generate at most 1
    // transition event (per type) each frame. No event means no change happened
    // and we skip iterating all entities.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited && !transition.allow_same_state_transitions {
        return;
    }
    for (entity, when) in &query {
        if (when.state_transition_evaluator)(transition) {
            commands.entity(entity).try_despawn();
        }
    }
}

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
///
/// For more advanced usecases see [`DespawnWhen`]
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
///
/// If the entity has already been despawned no warning will be emitted.
pub fn despawn_entities_on_exit_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &DespawnOnExit<S>), Allow<Disabled>>,
) {
    // We use the latest event, because state machine internals generate at most 1
    // transition event (per type) each frame. No event means no change happened
    // and we skip iterating all entities.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited && !transition.allow_same_state_transitions {
        return;
    }
    let Some(exited) = &transition.exited else {
        return;
    };
    for (entity, exit) in &query {
        if exit.0 == *exited {
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
///
/// For more advanced usecases see [`DespawnWhen`]
#[derive(Component, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct DespawnOnEnter<S: States>(pub S);

impl<S: States + Default> Default for DespawnOnEnter<S> {
    fn default() -> Self {
        Self(S::default())
    }
}

/// Despawns entities marked with [`DespawnOnEnter<S>`] when their state
/// matches the world state.
///
/// If the entity has already been despawned no warning will be emitted.
pub fn despawn_entities_on_enter_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &DespawnOnEnter<S>), Allow<Disabled>>,
) {
    // We use the latest event, because state machine internals generate at most 1
    // transition event (per type) each frame. No event means no change happened
    // and we skip iterating all entities.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited && !transition.allow_same_state_transitions {
        return;
    }
    let Some(entered) = &transition.entered else {
        return;
    };
    for (entity, enter) in &query {
        if enter.0 == *entered {
            commands.entity(entity).try_despawn();
        }
    }
}

/// Entities marked with this component will be disabled
/// when a [`StateTransitionEvent<S>`] matching the given predicate is sent.
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
///     GameOver,
/// }
///
/// # #[derive(Component)]
/// # struct Player;
///
/// fn spawn_player(mut commands: Commands) {
///     commands.spawn((
///         DisableWhen::new(|transition| {
///             matches!(
///                 transition.entered,
///                 Some(GameState::MainMenu) | Some(GameState::GameOver)
///             )
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
///
/// See also [`DisableOnExit`] and [`DisableOnEnter`].
#[derive(Component)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct DisableWhen<S: States> {
    /// The predicate that is ran when message [`StateTransitionEvent<S>`] is sent.
    pub state_transition_evaluator:
        Box<dyn Fn(&StateTransitionEvent<S>) -> bool + Sync + Send + 'static>,
}

impl<S: States> DisableWhen<S> {
    /// Creates a [`DisableWhen`] for the given predicate.
    pub fn new(f: impl Fn(&StateTransitionEvent<S>) -> bool + Sync + Send + 'static) -> Self {
        Self {
            state_transition_evaluator: Box::new(f),
        }
    }
}

/// Disable entities marked with [`DisableWhen<S>`] when the state transition message matches their
/// predicate.
pub fn disable_entities_when_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &DisableWhen<S>), Allow<Disabled>>,
) {
    // We use the latest event, because state machine internals generate at most 1
    // transition event (per type) each frame. No event means no change happened
    // and we skip iterating all entities.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited && !transition.allow_same_state_transitions {
        return;
    }
    for (entity, when) in &query {
        if (when.state_transition_evaluator)(transition) {
            commands
                .entity(entity)
                .insert_recursive::<Children>(Disabled);
        }
    }
}

/// Entities marked with this component will be disabled
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
///         DisableOnExit(GameState::InGame),
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
///
/// For more advanced usecases see [`DisableWhen`]
#[derive(Component, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component, Clone))]
pub struct DisableOnExit<S: States>(pub S);

impl<S> Default for DisableOnExit<S>
where
    S: States + Default,
{
    fn default() -> Self {
        Self(S::default())
    }
}

/// Disables entities marked with [`DisableOnExit<S>`] when their state no
/// longer matches the world state.
pub fn disable_entities_on_exit_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &DisableOnExit<S>), Allow<Disabled>>,
) {
    // We use the latest event, because state machine internals generate at most 1
    // transition event (per type) each frame. No event means no change happened
    // and we skip iterating all entities.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited && !transition.allow_same_state_transitions {
        return;
    }
    let Some(exited) = &transition.exited else {
        return;
    };
    for (entity, exit) in &query {
        if exit.0 == *exited {
            commands
                .entity(entity)
                .insert_recursive::<Children>(Disabled);
        }
    }
}

/// Entities marked with this component will be disabled
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
///         DisableOnEnter(GameState::MainMenu),
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
///
/// For more advanced usecases see [`DisableWhen`]
#[derive(Component, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct DisableOnEnter<S: States>(pub S);

impl<S: States + Default> Default for DisableOnEnter<S> {
    fn default() -> Self {
        Self(S::default())
    }
}

/// Disables entities marked with [`DisableOnEnter<S>`] when their state
/// matches the world state.
pub fn disable_entities_on_enter_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &DisableOnEnter<S>), Allow<Disabled>>,
) {
    // We use the latest event, because state machine internals generate at most 1
    // transition event (per type) each frame. No event means no change happened
    // and we skip iterating all entities.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited && !transition.allow_same_state_transitions {
        return;
    }
    let Some(entered) = &transition.entered else {
        return;
    };
    for (entity, enter) in &query {
        if enter.0 == *entered {
            commands
                .entity(entity)
                .insert_recursive::<Children>(Disabled);
        }
    }
}

/// Entities marked with this component will be enabled
/// when a [`StateTransitionEvent<S>`] matching the given predicate is sent.
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
///     GameOver,
/// }
///
/// # #[derive(Component)]
/// # struct Player;
///
/// fn spawn_player(mut commands: Commands) {
///     commands.spawn((
///         EnableWhen::new(|transition| {
///             matches!(
///                 transition.exited,
///                 Some(GameState::MainMenu) | Some(GameState::GameOver)
///             )
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
///
/// See also [`EnableOnExit`] and [`EnableOnEnter`].
#[derive(Component)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct EnableWhen<S: States> {
    /// The predicate that is ran when message [`StateTransitionEvent<S>`] is sent.
    pub state_transition_evaluator:
        Box<dyn Fn(&StateTransitionEvent<S>) -> bool + Sync + Send + 'static>,
}

impl<S: States> EnableWhen<S> {
    /// Creates a [`EnableWhen`] for the given predicate.
    pub fn new(f: impl Fn(&StateTransitionEvent<S>) -> bool + Sync + Send + 'static) -> Self {
        Self {
            state_transition_evaluator: Box::new(f),
        }
    }
}

/// Enable entities marked with [`EnableWhen<S>`] when the state transition message matches their
/// predicate.
pub fn enable_entities_when_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &EnableWhen<S>), With<Disabled>>,
) {
    // We use the latest event, because state machine internals generate at most 1
    // transition event (per type) each frame. No event means no change happened
    // and we skip iterating all entities.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited && !transition.allow_same_state_transitions {
        return;
    }
    for (entity, when) in &query {
        if (when.state_transition_evaluator)(transition) {
            commands
                .entity(entity)
                .remove_recursive::<Children, Disabled>();
        }
    }
}

/// Entities marked with this component will be enabled
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
///         EnableOnExit(GameState::MainMenu),
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
///
/// For more advanced usecases see [`EnableWhen`]
#[derive(Component, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component, Clone))]
pub struct EnableOnExit<S: States>(pub S);

impl<S> Default for EnableOnExit<S>
where
    S: States + Default,
{
    fn default() -> Self {
        Self(S::default())
    }
}

/// Enables entities marked with [`EnableOnExit<S>`] when their state no
/// longer matches the world state.
pub fn enable_entities_on_exit_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &EnableOnExit<S>), With<Disabled>>,
) {
    // We use the latest event, because state machine internals generate at most 1
    // transition event (per type) each frame. No event means no change happened
    // and we skip iterating all entities.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited && !transition.allow_same_state_transitions {
        return;
    }
    let Some(exited) = &transition.exited else {
        return;
    };
    for (entity, exit) in &query {
        if exit.0 == *exited {
            commands
                .entity(entity)
                .remove_recursive::<Children, Disabled>();
        }
    }
}

/// Entities marked with this component will be enabled
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
///         EnableOnEnter(GameState::InGame),
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
///
/// For more advanced usecases see [`EnableWhen`]
#[derive(Component, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct EnableOnEnter<S: States>(pub S);

impl<S: States + Default> Default for EnableOnEnter<S> {
    fn default() -> Self {
        Self(S::default())
    }
}

/// Enables entities marked with [`EnableOnEnter<S>`] when their state
/// matches the world state.
pub fn enable_entities_on_enter_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &EnableOnEnter<S>), With<Disabled>>,
) {
    // We use the latest event, because state machine internals generate at most 1
    // transition event (per type) each frame. No event means no change happened
    // and we skip iterating all entities.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited && !transition.allow_same_state_transitions {
        return;
    }
    let Some(entered) = &transition.entered else {
        return;
    };
    for (entity, enter) in &query {
        if enter.0 == *entered {
            commands
                .entity(entity)
                .remove_recursive::<Children, Disabled>();
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

    #[test]
    fn despawn_on_exit_same_state_transition() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
        enum State {
            On,
        }

        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        app.insert_state(State::On);
        app.update();

        assert_eq!(
            app.world()
                .resource::<bevy_state::state::State<State>>()
                .get(),
            &State::On
        );

        let entity = app.world_mut().spawn(DespawnOnExit(State::On)).id();
        assert!(app.world().get_entity(entity).is_ok());

        app.world_mut().commands().set_state(State::On);
        app.update();

        assert_eq!(
            app.world()
                .resource::<bevy_state::state::State<State>>()
                .get(),
            &State::On
        );
        // entity was despawned on exit, despite setting the state to the same state.
        // this is because "set_state" runs state transitions even if
        // the next state and the previous are equal.
        assert!(app.world().get_entity(entity).is_err());

        let entity = app.world_mut().spawn(DespawnOnExit(State::On)).id();
        assert!(app.world().get_entity(entity).is_ok());

        app.world_mut().commands().set_state_if_neq(State::On);
        app.update();

        assert_eq!(
            app.world()
                .resource::<bevy_state::state::State<State>>()
                .get(),
            &State::On
        );
        // entity was not despawned on exit
        // this is because "set_state_if_neq" skips state transitions since
        // the app's next state is the same as its previous.
        assert!(app.world().get_entity(entity).is_ok());
    }
}
