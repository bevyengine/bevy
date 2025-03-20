#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EventReader,
    system::{Commands, Query},
};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

use crate::state::{StateTransitionEvent, States};

/// Entities marked with this component will be removed
/// when the world's state of the matching type no longer matches the supplied value.
///
/// To enable this feature remember to add the attribute `#[states(scoped_entities)]` when deriving [`States`].
/// It's also possible to enable it when adding the state to an app with [`enable_state_scoped_entities`](crate::app::AppExtStates::enable_state_scoped_entities).
///
/// ```
/// use bevy_state::prelude::*;
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::system::ScheduleSystem;
///
/// #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
/// #[states(scoped_entities)]
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
///         StateScoped(GameState::InGame),
///         Player
///     ));
/// }
///
/// # struct AppMock;
/// # impl AppMock {
/// #     fn init_state<S>(&mut self) {}
/// #     fn enable_state_scoped_entities<S>(&mut self) {}
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
pub struct StateScoped<S: States>(pub S);

impl<S> Default for StateScoped<S>
where
    S: States + Default,
{
    fn default() -> Self {
        Self(S::default())
    }
}

/// Removes entities marked with [`StateScoped<S>`]
/// when their state no longer matches the world state.
pub fn clear_state_scoped_entities<S: States>(
    mut commands: Commands,
    mut transitions: EventReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &StateScoped<S>)>,
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
