//! Tools for convenient integration testing of the ECS.
//!
//! Each of these methods has a corresponding method on `World`.

use crate::App;
use bevy_ecs::component::Component;
use bevy_ecs::query::{FilterFetch, WorldQuery};
use bevy_ecs::system::IntoSystem;
use bevy_ecs::system::Resource;
use std::fmt::Debug;

impl App {
    /// Asserts that that the current value of the resource `R` is `value`
    ///
    /// # Example
    /// ```rust
    /// # use bevy_app::App;
    /// # use bevy_ecs::prelude::*;
    ///
    /// // The resource we want to check the value of
    /// #[derive(PartialEq, Debug)]
    /// enum Toggle {
    ///     On,
    ///     Off,
    /// }
    ///
    /// let mut app = App::new();
    ///
    /// // This system modifies our resource
    /// fn toggle_off(mut toggle: ResMut<Toggle>) {
    ///     *toggle = Toggle::Off;
    /// }
    ///
    /// app.insert_resource(Toggle::On).add_system(toggle_off);
    ///
    /// // Checking that the resource was initialized correctly
    /// app.assert_resource_eq(Toggle::On);
    ///
    /// // Run the `Schedule` once, causing our system to trigger
    /// app.update();
    ///
    /// // Checking that our resource was modified correctly
    /// app.assert_resource_eq(Toggle::Off);
    /// ```
    pub fn assert_resource_eq<R: Resource + PartialEq + Debug>(&self, value: R) {
        self.world.assert_resource_eq(value);
    }

    /// Asserts that that the current value of the non-send resource `NS` is `value`
    pub fn assert_nonsend_resource_eq<NS: 'static + PartialEq + Debug>(&self, value: NS) {
        self.world.assert_nonsend_resource_eq(value);
    }

    /// Asserts that all components of type `C` returned by a query with the filter `F` will equal `value`
    ///
    /// This is commonly used with the corresponding `query_len` method to ensure that the returned query is not empty.
    ///
    /// WARNING: because we are constructing the query from scratch,
    /// [`Changed`](crate::query::Changed) and [`Added`](crate::query::Added) filters
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
    /// #[derive(Component)]
    /// struct Life(usize);
    ///
    /// let mut app = App::new();
    ///
    /// fn spawn_player(mut commands: Commands){
    ///     commands.spawn().insert(Life(8)).insert(Player);
    /// }
    ///
    /// fn regenerate_life(query: Query<&Life>){
    ///     for life in query.iter(){
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
    /// app.assert_component_eq<()>(Life(9));
    ///
    /// app.update();
    /// // Because all of our entities with the `Life` component also
    /// // have the `Player` component, these will be equivalent.
    /// app.assert_component_eq<With<Player>>(Life(10));
    ///
    /// app.update();
    /// // Check that life regeneration caps at 10, as intended
    /// // Filtering by the component type you're looking for is useless,
    /// // but it's helpful to demonstrate composing query filters here
    /// app.assert_component_eq<(With<Player>, With<Life>)>(Life(10));
    /// ```
    pub fn assert_component_eq<C, F>(&mut self, value: &C)
    where
        C: Component + PartialEq + Debug,
        F: WorldQuery,
        <F as WorldQuery>::Fetch: FilterFetch,
    {
        self.world.assert_component_eq::<C, F>(value);
    }

    /// Asserts that when the supplied `system` is run on the world, its output will be `Ok`
    ///
    /// The `system` must return a `Result`: if the return value is an error the app will panic.
    ///
    /// For more sophisticated error-handling, consider adding the system directly to the schedule
    /// and using [system chaining](bevy_ecs::prelude::IntoChainSystem) to handle the result yourself.
    ///
    /// WARNING: [`Changed`](bevy_ecs::query::Changed) and [`Added`](bevy_ecs::query::Added) filters
    /// are computed relative to "the last time this system ran".
    /// Because we are running a new system; these filters will always be true.
    ///
    /// # Example
    /// ```rust
    /// # use bevy_app::App;
    /// # use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Player;
    ///
    /// #[derive(Component)]
    /// struct Life(usize);
    ///
    /// #[derive(Component)]
    /// struct Dead;
    ///
    /// let mut app = App::new();
    ///
    /// fn spawn_player(mut commands: Commands){
    ///     commands.spawn().insert(Life(10)).insert(Player);
    /// }
    ///
    /// fn massive_damage(mut query: Query<&mut Life>){
    ///     for mut life in query.iter_mut(){
    ///         life.0 -= 9001;
    ///     }
    /// }
    ///
    /// fn kill_units(query: Query<(Entity, &Life)>, mut commands: Commands){
    ///     for (entity, life) in query.iter(){
    ///         if life.0 == 0 {
    /// 		    commands.entity(entity).insert(Dead);
    /// 	    }
    ///     }
    /// }
    ///
    /// app.add_startup_system(spawn_player)
    ///    .add_system(massive_damage)
    ///    .add_system(kill_units);
    ///
    /// // Run the `Schedule` once, causing both our startup systems
    /// // and ordinary systems to run once
    /// app.update();
    ///
    /// enum DeathError {
    /// 	ZeroLifeIsNotDead,
    /// 	DeadWithNonZeroLife,
    /// }
    ///
    /// // Run a complex assertion on the world using a system
    /// fn zero_life_is_dead(query: Query<(&Life, Option<&Dead>)>) -> Result<(), DeathError> {
    ///     for (life, maybe_dead) in query.iter(){
    ///        if life.0 == 0 {
    ///            if maybe_dead.is_none(){
    ///                return Err(DeathError::ZeroLifeIsNotDead);
    ///            }
    ///         }
    ///
    ///         if maybe_dead.is_some(){
    ///             if life.0 != 0 {
    ///                return Err(DeathError::DeadWithNonZeroLife);
    ///             }
    ///         }
    ///     }
    /// 	// None of our checks failed, so our world state is clean
    ///     true
    /// }
    ///
    /// app.update();
    /// app.assert_system(zero_life_is_dead, None);
    /// ```
    pub fn assert_system<T: 'static, E: 'static, SystemParams>(
        &mut self,
        system: impl IntoSystem<(), Result<T, E>, SystemParams>,
    ) {
        self.world.assert_system(system);
    }
}
