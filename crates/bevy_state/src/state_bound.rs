use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Commands, Query},
};
use bevy_hierarchy::DespawnRecursiveExt;

use crate::state::States;

/// Entities marked with this component will be removed
/// when the provided value no longer matches the world state.
///
/// To enable this feature, register the [`clear_state_bound_entities`]
/// system for selected states.
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
/// #     fn add_systems<S, M>(&mut self, schedule: S, systems: impl IntoSystemConfigs<M>) {}
/// # }
/// # struct Update;
/// # let mut app = AppMock;
///
/// app.add_systems(Update, clear_state_bound_entities::<GameState>);
/// app.add_systems(OnEnter(GameState::InGame), spawn_player);
/// ```
#[derive(Component)]
pub struct StateBound<S: States>(pub S);

/// Removes entities marked with [`StateBound<S>`]
/// when their state no longer matches the world state.
pub fn clear_state_bound_entities<S: States>(
    state: S,
) -> impl Fn(Commands, Query<(Entity, &StateBound<S>)>) {
    move |mut commands, query| {
        for (entity, bound) in &query {
            if bound.0 == state {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}
