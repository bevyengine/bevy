//! Tools for convenient integration testing of the ECS.
//!
//! Each of these methods has a corresponding method on `World`.

use crate::App;
use bevy_ecs::component::Component;
use bevy_ecs::query::{FilterFetch, WorldQuery};
use std::fmt::Debug;

impl App {
    /// Asserts that all components of type `C` returned by a query with the filter `F` will equal `value`
    ///
    /// This is commonly used with the corresponding `query_len` method to ensure that the returned query is not empty.
    ///
    /// WARNING: because we are constructing the query from scratch,
    /// [`Changed`](bevy_ecs::query::Changed) and [`Added`](bevy_ecs::query::Added) filters
    /// will always return true.
    ///
    /// # Example
    /// ```rust
    /// # use bevy_app::App;
    /// # use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Player;
    ///
    /// #[derive(Component, Debug, PartialEq)]
    /// struct Life(usize);
    ///
    /// let mut app = App::new();
    ///
    /// fn spawn_player(mut commands: Commands){
    ///     commands.spawn().insert(Life(8)).insert(Player);
    /// }
    ///
    /// fn regenerate_life(mut query: Query<&mut Life>){
    ///     for mut life in query.iter_mut(){
    ///         if life.0 < 10 {
    ///             life.0 += 1;
    ///         }
    ///     }
    /// }
    ///
    /// app.add_startup_system(spawn_player).add_system(regenerate_life);
    ///
    /// // Run the `Schedule` once, causing our startup system to run
    /// // and life to regenerate once
    /// app.update();
    /// // The `()` value for `F` will result in an unfiltered query
    /// app.assert_component_eq::<Life, ()>(&Life(9));
    ///
    /// app.update();
    /// // Because all of our entities with the `Life` component also
    /// // have the `Player` component, these will be equivalent.
    /// app.assert_component_eq::<Life, With<Player>>(&Life(10));
    ///
    /// app.update();
    /// // Check that life regeneration caps at 10, as intended
    /// // Filtering by the component type you're looking for is useless,
    /// // but it's helpful to demonstrate composing query filters here
    /// app.assert_component_eq::<Life, (With<Player>, With<Life>)>(&Life(10));
    /// ```
    pub fn assert_component_eq<C, F>(&mut self, value: &C)
    where
        C: Component + PartialEq + Debug,
        F: WorldQuery,
        <F as WorldQuery>::Fetch: FilterFetch,
    {
        self.world.assert_component_eq::<C, F>(value);
    }
}
