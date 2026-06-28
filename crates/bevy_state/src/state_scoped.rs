use alloc::{boxed::Box, vec, vec::Vec};

#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    entity_disabling::Disabled,
    hierarchy::Children,
    lifecycle::{Insert, Remove},
    message::MessageReader,
    observer::On,
    query::{Allow, With},
    system::{Commands, Query, Res},
    world::World,
};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

use crate::state::{State, StateTransitionEvent, States};

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

/// Marks this entity as owning its [`Disabled`] state.
/// When present, parent-driven enable and disable propagation skips this entity
/// and its descendants.
///
/// Automatically required by state-driven disabling components
/// i.e., ([`EnabledIn`], [`DisabledIn`], [`EnabledIf`], [`DisabledIf`]).
/// Can be inserted manually.
///
/// If you only want to block re-enabling while still allowing a parent disable to
/// propagate to this entity, use [`DisabledSelf`] instead.
/// ```
/// # use bevy_app::Startup;
/// use bevy_state::prelude::*;
/// use bevy_ecs::{prelude::*, system::ScheduleSystem, entity_disabling::Disabled};
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     InGame,
/// }
///
/// # #[derive(Component)]
/// # struct ParentEntity;
/// # #[derive(Component)]
/// # struct ShieldedChild;
/// fn spawn_parent_entity(mut commands: Commands) {
///     commands.spawn((
///         ParentEntity,
///         EnabledIn(GameState::MainMenu),
///         children![(
///             // This entity and its descendants will be ignored by parent state transitions.
///             ShieldedChild,
///             OwnsDisabled,
///         )]
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
/// app.add_systems(Startup, spawn_parent_entity);
/// ```
///
/// See also [`DisabledSelf`], [`EnabledIn`], [`DisabledIn`], [`EnabledIf`], and [`DisabledIf`].
#[derive(Component, Clone, Default)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Clone, Default)
)]
pub struct OwnsDisabled;

/// Marks this entity as independently disabled.
/// When present, parent-driven **enable** propagation skips this entity and its
/// descendants, but parent-driven **disable** propagation still applies.
///
/// Use this when you want a child to be disabled together with its parent, but you
/// do not want the parent re-enabling to automatically clear [`Disabled`] from this
/// entity or its descendants.
///
/// Note: state-driven disabling components require [`OwnsDisabled`], which blocks
/// both disable and enable propagation. If an entity has both [`OwnsDisabled`] and
/// [`DisabledSelf`], [`OwnsDisabled`] takes precedence.
/// ```
/// # use bevy_app::Startup;
/// use bevy_state::prelude::*;
/// use bevy_ecs::{prelude::*, system::ScheduleSystem, entity_disabling::Disabled};
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     InGame,
/// }
///
/// # #[derive(Component)]
/// # struct ParentEntity;
/// # #[derive(Component)]
/// # struct ShieldedChild;
/// fn spawn_parent_entity(mut commands: Commands) {
///     commands.spawn((
///         ParentEntity,
///         EnabledIn(GameState::MainMenu),
///         children![(
///             // This entity will be disabled when its parent is disabled, but it
///             // will stay disabled when the parent is re-enabled.
///             ShieldedChild,
///             DisabledSelf,
///         )]
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
/// app.add_systems(Startup, spawn_parent_entity);
/// ```
///
/// See also [`OwnsDisabled`], [`EnabledIn`], [`DisabledIn`], [`EnabledIf`], and [`DisabledIf`].
#[derive(Component, Clone, Default)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Clone, Default)
)]
pub struct DisabledSelf;

/// Removes [`OwnsDisabled`] component from an entity
/// when its managing state-driven disabling components are removed
/// i.e. ([`EnabledIn`], [`DisabledIn`], [`EnabledIf`], [`DisabledIf`]).
pub fn on_state_disabled_component_remove<C: Component>(on: On<Remove, C>, mut commands: Commands) {
    let mut entity = commands.entity(on.entity);
    entity.remove::<OwnsDisabled>();
}

/// Entities marked with this component will be automatically enabled
/// when the world is in the given state, and disabled otherwise.
///
/// This component takes ownership of adding or removing the entity's [`Disabled`] component
/// at state transitions and component insertion.
/// At component removal, [`Disabled`] is left as is.
///
/// # Note
/// System is added on state registration, so `Res<State<S>>` should always exist.
///
/// ```
/// # use bevy_app::Startup;
/// use bevy_state::prelude::*;
/// use bevy_ecs::{prelude::*, system::ScheduleSystem};
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     InGame,
/// }
/// # #[derive(Component)]
/// # struct Player;
/// fn spawn_player(mut commands: Commands) {
///     commands.spawn((
///         EnabledIn(GameState::InGame),
///         Player,
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
/// app.add_systems(Startup, spawn_player);
/// ```
///
/// Use [`EnableOnEnter`] and [`DisableOnExit`] separately if you need finer control.
/// See also [`EnabledIf`] and [`DisabledIn`].
#[derive(Component, Clone)]
#[require(OwnsDisabled)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component, Clone))]
pub struct EnabledIn<S: States>(pub S);

impl<S: States + Default> Default for EnabledIn<S> {
    fn default() -> Self {
        Self(S::default())
    }
}

/// Enables or disables entities marked with [`EnabledIn<S>`] on state transition.
/// Removes [`Disabled`] when entering the target state, inserts it when exiting.
pub fn update_enabled_in_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &EnabledIn<S>), Allow<Disabled>>,
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
    for (entity, enabled_in) in &query {
        if transition.entered.as_ref() == Some(&enabled_in.0) {
            propagate_enable(&mut commands, entity);
        } else if transition.exited.as_ref() == Some(&enabled_in.0) {
            propagate_disable(&mut commands, entity);
        }
    }
}

/// On [`EnabledIn<S>`] insertion updates [`Disabled`] component of that entity based on current state.
pub fn on_enabled_in_insert<S: States>(
    on: On<Insert, EnabledIn<S>>,
    mut commands: Commands,
    current_state: Res<State<S>>,
    query: Query<&EnabledIn<S>, Allow<Disabled>>,
) {
    let entity = on.entity;
    let Ok(enabled_in) = query.get(entity) else {
        return;
    };
    let in_target_state = current_state.get() == &enabled_in.0;
    if in_target_state {
        propagate_enable(&mut commands, entity);
    } else {
        propagate_disable(&mut commands, entity);
    }
}

/// Entities marked with this component will be automatically disabled
/// when the world is in the given state, and enabled otherwise.
///
/// This component takes ownership of adding or removing the entity's [`Disabled`] component
/// at state transitions and component insertion.
/// At component removal, [`Disabled`] is left as is.
///
/// # Note
/// System is added on state registration, so `Res<State<S>>` should always exist.
///
/// ```
/// # use bevy_app::Startup;
/// use bevy_state::prelude::*;
/// use bevy_ecs::{prelude::*, system::ScheduleSystem};
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     InGame,
/// }
/// # #[derive(Component)]
/// # struct Player;
/// fn spawn_player(mut commands: Commands) {
///     commands.spawn((
///         DisabledIn(GameState::MainMenu),
///         Player,
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
/// app.add_systems(Startup, spawn_player);
/// ```
///
/// Use [`DisableOnEnter`] and [`EnableOnExit`] separately if you need finer control.
/// See also [`DisabledIf`] and [`EnabledIn`].
#[derive(Component, Clone)]
#[require(OwnsDisabled)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component, Clone))]
pub struct DisabledIn<S: States>(pub S);

impl<S: States + Default> Default for DisabledIn<S> {
    fn default() -> Self {
        Self(S::default())
    }
}

/// Disables or enables entities marked with [`DisabledIn<S>`] on state transition.
/// Inserts [`Disabled`] when entering the target state, removes it when exiting.
pub fn update_disabled_in_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &DisabledIn<S>), Allow<Disabled>>,
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
    for (entity, disabled_in) in &query {
        if transition.entered.as_ref() == Some(&disabled_in.0) {
            propagate_disable(&mut commands, entity);
        } else if transition.exited.as_ref() == Some(&disabled_in.0) {
            propagate_enable(&mut commands, entity);
        }
    }
}

/// On [`DisabledIn<S>`] insertion updates [`Disabled`] component of that entity based on current state.
pub fn on_disabled_in_insert<S: States>(
    on: On<Insert, DisabledIn<S>>,
    mut commands: Commands,
    current_state: Res<State<S>>,
    query: Query<&DisabledIn<S>, Allow<Disabled>>,
) {
    let entity = on.entity;
    let Ok(disabled_in) = query.get(entity) else {
        return;
    };
    let in_target_state = current_state.get() == &disabled_in.0;
    if in_target_state {
        propagate_disable(&mut commands, entity);
    } else {
        propagate_enable(&mut commands, entity);
    }
}

/// Entities marked with this component will be automatically enabled
/// when the predicate returns `true` for the current state, disabled otherwise.
///
/// This component takes ownership of adding or removing the entity's [`Disabled`] component
/// at state transitions and component insertion.
/// At component removal, [`Disabled`] is left as is.
///
/// # Note
/// System is added on state registration, so `Res<State<S>>` should always exist.
///
/// ```
/// # use bevy_app::Startup;
/// use bevy_state::prelude::*;
/// use bevy_ecs::{prelude::*, system::ScheduleSystem};
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     InGame,
/// }
/// # #[derive(Component)]
/// # struct Player;
/// fn spawn_player(mut commands: Commands) {
///     commands.spawn((
///         EnabledIf::new(|s|  matches!(
///             s,
///             GameState::InGame
///         )),
///         Player,
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
/// app.add_systems(Startup, spawn_player);
/// ```
///
/// See also [`DisabledIf`] and [`EnabledIn`].
#[derive(Component)]
#[require(OwnsDisabled)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct EnabledIf<S: States> {
    /// The predicate used to determine if the entity should be enabled for the given state.
    pub predicate: Box<dyn Fn(&S) -> bool + Sync + Send + 'static>,
}

impl<S: States> EnabledIf<S> {
    /// Creates an [`EnabledIf`] for the given predicate.
    pub fn new(f: impl Fn(&S) -> bool + Sync + Send + 'static) -> Self {
        Self {
            predicate: Box::new(f),
        }
    }
}

/// Enables or disables entities marked with [`EnabledIf<S>`] based on their predicate evaluated against the entered state.
/// Removes [`Disabled`] when `true` and inserts it when `false`.
pub fn update_enabled_if_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &EnabledIf<S>), Allow<Disabled>>,
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
    for (entity, enabled_if) in &query {
        let should_enable = transition
            .entered
            .as_ref()
            .is_some_and(|s| (enabled_if.predicate)(s));
        if should_enable {
            propagate_enable(&mut commands, entity);
        } else {
            propagate_disable(&mut commands, entity);
        }
    }
}

/// On [`EnabledIf<S>`] insertion updates [`Disabled`] component depending on predicate evaluated against the current state.
pub fn on_enabled_if_insert<S: States>(
    on: On<Insert, EnabledIf<S>>,
    mut commands: Commands,
    current_state: Res<State<S>>,
    query: Query<&EnabledIf<S>, Allow<Disabled>>,
) {
    let entity = on.entity;
    let Ok(enabled_if) = query.get(entity) else {
        return;
    };
    let should_enable = (enabled_if.predicate)(current_state.get());
    if should_enable {
        propagate_enable(&mut commands, entity);
    } else {
        propagate_disable(&mut commands, entity);
    }
}

/// Entities marked with this component will be automatically disabled
/// when the predicate returns `true` for the current state, enabled otherwise.
///
/// This component takes ownership of adding or removing the entity's [`Disabled`] component
/// at state transitions and component insertion.
/// At component removal, [`Disabled`] is left as is.
///
/// # Note
/// System is added on state registration, so `Res<State<S>>` should always exist.
///
/// ```
/// # use bevy_app::Startup;
/// use bevy_state::prelude::*;
/// use bevy_ecs::{prelude::*, system::ScheduleSystem};
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// enum GameState {
///     #[default]
///     MainMenu,
///     InGame,
///     Settings,
/// }
/// # #[derive(Component)]
/// # struct Player;
/// fn spawn_player(mut commands: Commands) {
///     commands.spawn((
///         DisabledIf::new(|s|  matches!(
///             s,
///             GameState::MainMenu | GameState::Settings
///         )),
///         Player,
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
/// app.add_systems(Startup, spawn_player);
/// ```
///
/// See also [`EnabledIf`] and [`DisabledIn`].
#[derive(Component)]
#[require(OwnsDisabled)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct DisabledIf<S: States> {
    /// The predicate used to determine if the entity should be disabled for the given state.
    pub predicate: Box<dyn Fn(&S) -> bool + Sync + Send + 'static>,
}

impl<S: States> DisabledIf<S> {
    /// Creates a [`DisabledIf`] for the given predicate.
    pub fn new(f: impl Fn(&S) -> bool + Sync + Send + 'static) -> Self {
        Self {
            predicate: Box::new(f),
        }
    }
}

/// Disables or enables entities marked with [`DisabledIf<S>`] based on their predicate evaluated against the entered state.
/// Inserts [`Disabled`] when `true` and removes it when `false`.
pub fn update_disabled_if_state<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &DisabledIf<S>), Allow<Disabled>>,
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
    for (entity, disabled_if) in &query {
        let should_disable = transition
            .entered
            .as_ref()
            .is_some_and(|s| (disabled_if.predicate)(s));
        if should_disable {
            propagate_disable(&mut commands, entity);
        } else {
            propagate_enable(&mut commands, entity);
        }
    }
}

/// On [`DisabledIf<S>`] insertion updates [`Disabled`] component depending on predicate evaluated against the current state.
pub fn on_disabled_if_insert<S: States>(
    on: On<Insert, DisabledIf<S>>,
    mut commands: Commands,
    current_state: Res<State<S>>,
    query: Query<&DisabledIf<S>, Allow<Disabled>>,
) {
    let entity = on.entity;
    let Ok(disabled_if) = query.get(entity) else {
        return;
    };
    let should_disable = (disabled_if.predicate)(current_state.get());
    if should_disable {
        propagate_disable(&mut commands, entity);
    } else {
        propagate_enable(&mut commands, entity);
    }
}

/// Propagates enabling to `entity` and its descendants, stopping at [`OwnsDisabled`]
/// and [`DisabledSelf`].
fn propagate_enable(commands: &mut Commands, entity: Entity) {
    commands.queue(move |world: &mut World| {
        let mut stack = vec![entity];
        while let Some(current) = stack.pop() {
            let Ok(mut entity_mut) = world.get_entity_mut(current) else {
                continue;
            };
            entity_mut.remove::<Disabled>();
            let children: Vec<Entity> = entity_mut
                .get::<Children>()
                .map(|c| c.iter().copied().collect())
                .unwrap_or_default();
            for child in children {
                if world.get::<OwnsDisabled>(child).is_none()
                    && world.get::<DisabledSelf>(child).is_none()
                {
                    stack.push(child);
                }
            }
        }
    });
}

/// Propagates disabling to `entity` and its descendants, stopping at [`OwnsDisabled`].
fn propagate_disable(commands: &mut Commands, entity: Entity) {
    commands.queue(move |world: &mut World| {
        let mut stack = vec![entity];
        while let Some(current) = stack.pop() {
            let Ok(mut entity_mut) = world.get_entity_mut(current) else {
                continue;
            };
            entity_mut.insert(Disabled);
            let children: Vec<Entity> = entity_mut
                .get::<Children>()
                .map(|c| c.iter().copied().collect())
                .unwrap_or_default();
            for child in children {
                if world.get::<OwnsDisabled>(child).is_none() {
                    stack.push(child);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use bevy_app::App;

    use crate::{
        app::{AppExtStates, StatesPlugin},
        prelude::CommandsStatesExt,
    };

    fn is_disabled<T: Component>(world: &mut World) -> bool {
        world
            .query_filtered::<&Disabled, (With<T>, Allow<Disabled>)>()
            .single(world)
            .is_ok()
    }

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

        app.world_mut().commands().set_state_if_different(State::On);
        app.update();

        assert_eq!(
            app.world()
                .resource::<bevy_state::state::State<State>>()
                .get(),
            &State::On
        );
        // entity was not despawned on exit
        // this is because "set_state_if_different" skips state transitions since
        // the app's next state is the same as its previous.
        assert!(app.world().get_entity(entity).is_ok());
    }

    #[test]
    fn enabled_in_spawns_outside_target_state() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
        enum State {
            On,
            Off,
        }
        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        app.insert_state(State::Off);
        app.update();
        let entity = app.world_mut().spawn(EnabledIn(State::On)).id();
        assert!(app.world().get::<Disabled>(entity).is_some());

        app.world_mut().commands().set_state(State::On);
        app.update();
        assert!(app.world().get::<Disabled>(entity).is_none());

        app.world_mut().commands().set_state(State::Off);
        app.update();
        assert!(app.world().get::<Disabled>(entity).is_some());

        // Cleanup observer should remove the `OwnsDisabled` component.
        app.world_mut()
            .entity_mut(entity)
            .remove::<EnabledIn<State>>();
        assert!(app.world().get::<OwnsDisabled>(entity).is_none());
    }

    #[test]
    fn disabled_in_spawns_inside_target_state() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
        enum State {
            On,
            Off,
        }
        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        app.insert_state(State::Off);
        app.update();
        let entity = app.world_mut().spawn(DisabledIn(State::Off)).id();
        assert!(app.world().get::<Disabled>(entity).is_some());

        app.world_mut().commands().set_state(State::On);
        app.update();
        assert!(app.world().get::<Disabled>(entity).is_none());
    }

    #[test]
    fn enabled_if_spawns_outside_target_state() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
        enum State {
            On,
            Off,
            Limbo,
        }

        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        app.insert_state(State::Off);
        app.update();
        let entity = app
            .world_mut()
            .spawn(EnabledIf::new(|state: &State| {
                matches!(state, State::On | State::Limbo) // predicate evals to false - Disabled
            }))
            .id();
        assert!(app.world().get::<Disabled>(entity).is_some());

        // predicate evals to true - Enabled
        app.world_mut().commands().set_state(State::Limbo);
        app.update();
        assert!(app.world().get::<Disabled>(entity).is_none());

        // predicate evals to true - Enabled
        app.world_mut().commands().set_state(State::On);
        app.update();
        assert!(app.world().get::<Disabled>(entity).is_none());
    }

    #[test]
    fn disabled_if_spawns_inside_target_state() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
        enum State {
            On,
            Off,
            Limbo,
        }
        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        app.insert_state(State::Off);
        app.update();
        let entity = app
            .world_mut()
            .spawn(DisabledIf::new(|state: &State| {
                matches!(state, State::Off | State::Limbo) // predicate evals to true - Disabled
            }))
            .id();
        assert!(app.world().get::<Disabled>(entity).is_some());

        // predicate evals to false - Enabled
        app.world_mut().commands().set_state(State::On);
        app.update();
        assert!(app.world().get::<Disabled>(entity).is_none());

        // predicate evals to true - Disabled
        app.world_mut().commands().set_state(State::Limbo);
        app.update();
        assert!(app.world().get::<Disabled>(entity).is_some());
    }

    #[test]
    fn enabled_in_disabled_in_propagation() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
        enum State {
            Off,
            Limbo,
            On,
        }

        #[derive(Component)]
        struct Entity1;
        #[derive(Component)]
        struct Entity2;
        #[derive(Component)]
        struct Entity3;
        #[derive(Component)]
        struct Entity4;

        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        app.insert_state(State::Off);
        app.update();

        /*
        | Entity  | Component           | Off      | Limbo    | On       |
        |---------|---------------------|----------|----------|----------|
        | Entity1 | `EnabledIn(On)`     | disabled | disabled | enabled  |
        | Entity2 | `DisabledIn(On)`    | enabled  | enabled  | disabled |
        | Entity3 | `OwnsDisabled`      | enabled  | enabled  | enabled  |
        | Entity4 | `DisabledIn(Limbo)` | enabled  | disabled | enabled  |
        */

        app.world_mut().spawn((
            Entity1,
            EnabledIn(State::On),
            bevy_ecs::children![(
                Entity2,
                DisabledIn(State::On),
                bevy_ecs::children![(
                    Entity3,
                    OwnsDisabled,
                    bevy_ecs::children![(Entity4, DisabledIn(State::Limbo))]
                )]
            )],
        ));

        // Initialized as State::Off
        assert!(is_disabled::<Entity1>(app.world_mut()));
        assert!(!is_disabled::<Entity2>(app.world_mut()));
        assert!(!is_disabled::<Entity3>(app.world_mut()));
        assert!(!is_disabled::<Entity4>(app.world_mut()));

        // Switch to State::Limbo
        app.world_mut().commands().set_state(State::Limbo);
        app.update();

        assert!(is_disabled::<Entity1>(app.world_mut()));
        assert!(!is_disabled::<Entity2>(app.world_mut()));
        assert!(!is_disabled::<Entity3>(app.world_mut()));
        assert!(is_disabled::<Entity4>(app.world_mut()));

        // Switch to State::On
        app.world_mut().commands().set_state(State::On);
        app.update();

        assert!(!is_disabled::<Entity1>(app.world_mut()));
        assert!(is_disabled::<Entity2>(app.world_mut()));
        assert!(!is_disabled::<Entity3>(app.world_mut()));
        assert!(!is_disabled::<Entity4>(app.world_mut()));
    }

    #[test]
    fn disabled_self_blocks_re_enable_but_not_disable() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States)]
        enum State {
            Off,
            On,
        }

        #[derive(Component)]
        struct Parent;
        #[derive(Component)]
        struct Child;
        #[derive(Component)]
        struct Grandchild;

        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        app.insert_state(State::On);
        app.update();

        app.world_mut().spawn((
            Parent,
            EnabledIn(State::On),
            bevy_ecs::children![(Child, DisabledSelf, bevy_ecs::children![(Grandchild)])],
        ));

        // In State::On the whole hierarchy is enabled.
        assert!(!is_disabled::<Parent>(app.world_mut()));
        assert!(!is_disabled::<Child>(app.world_mut()));
        assert!(!is_disabled::<Grandchild>(app.world_mut()));

        // Switch to State::Off: parent is disabled, disable propagates through DisabledSelf.
        app.world_mut().commands().set_state(State::Off);
        app.update();

        assert!(is_disabled::<Parent>(app.world_mut()));
        assert!(is_disabled::<Child>(app.world_mut()));
        assert!(is_disabled::<Grandchild>(app.world_mut()));

        // Switch back to State::On: parent is enabled, but DisabledSelf blocks re-enable.
        app.world_mut().commands().set_state(State::On);
        app.update();

        assert!(!is_disabled::<Parent>(app.world_mut()));
        assert!(is_disabled::<Child>(app.world_mut()));
        assert!(is_disabled::<Grandchild>(app.world_mut()));
    }
}
