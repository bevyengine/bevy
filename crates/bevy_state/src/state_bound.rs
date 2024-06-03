use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EventReader,
    system::{Commands, Query},
};
use bevy_hierarchy::DespawnRecursiveExt;

use crate::state::{StateTransitionEvent, States};

/// Entities marked with this component will be removed
/// when the world's state of the matching type no longer matches the supplied value.
///
/// To enable this feature remember configure your application
/// with [`enable_state_bound_entities`](crate::app::AppExtStates::enable_state_bound_entities) on your state(s) of choice.
///
/// ```
/// use bevy_state::prelude::*;
/// use bevy_ecs::prelude::*;
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
///         StateBound(GameState::InGame),
///         Player
///     ));
/// }
///
/// # struct AppMock;
/// # impl AppMock {
/// #     fn init_state<S>(&mut self) {}
/// #     fn add_state_bound<S>(&mut self) {}
/// #     fn add_systems<S, M>(&mut self, schedule: S, systems: impl IntoSystemConfigs<M>) {}
/// # }
/// # struct Update;
/// # let mut app = AppMock;
///
/// app.init_state::<GameState>();
/// app.add_state_bound::<GameState>();
/// app.add_systems(OnEnter(GameState::InGame), spawn_player);
/// ```
#[derive(Component)]
pub struct StateBound<S: States>(pub S);

/// Removes entities marked with [`StateBound<S>`]
/// when their state no longer matches the world state.
pub fn clear_state_bound_entities<S: States>(
    mut commands: Commands,
    mut transitions: EventReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &StateBound<S>)>,
) {
    // We use the latest event, because state machine internals generate at most 1
    // transition event (per type) each frame. No event means no change happened
    // and we skip iterating all entities.
    let Some(transition) = transitions.read().last() else {
        return;
    };
    let Some(exited) = &transition.exited else {
        return;
    };
    for (entity, binding) in &query {
        if binding.0 == *exited {
            commands.entity(entity).despawn_recursive();
        }
    }
}
