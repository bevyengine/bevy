use crate::App;
use bevy_ecs::query::{FilterFetch, WorldQuery};
use bevy_ecs::system::IntoSystem;
use bevy_ecs::system::Resource;
use std::fmt::Debug;

impl App {
    /// Asserts that that the current value of the resource `R` is `value`
    ///
    /// # Example
    /// ```rust
    /// use bevy::prelude::*;
    ///
    /// // The resource we want to check the value of
    /// enum Toggle{
    ///     On,
    ///     Off,
    /// }
    ///
    /// let mut app = App::new();
    ///
    /// // This system modifies our resource
    /// fn toggle_off()
    ///
    /// app.insert_resource(Toggle::On).add_system(toggle_off);
    ///
    /// app.assert_resource_eq(Toggle::On);
    ///
    /// // Run the `Schedule` once, causing our system to trigger
    /// app.update();
    ///
    /// app.assert_resource_eq(Toggle::Off);
    /// ```
    pub fn assert_resource_eq<R: Resource + PartialEq + Debug>(&self, value: R) {
        self.world.assert_resource_eq(value);
    }

    /// Asserts that that the current value of the non-send resource `NS` is `value`
    pub fn assert_nonsend_resource_eq<NS: 'static + PartialEq + Debug>(&self, value: NS) {
        self.world.assert_nonsend_resource_eq(value);
    }

    /// Asserts that the number of entities returned by the query is exactly `n`
    pub fn assert_n_in_query<Q, F>(&mut self, n: usize)
    where
        Q: WorldQuery,
        F: WorldQuery,
        <F as WorldQuery>::Fetch: FilterFetch,
    {
        self.world.assert_n_in_query::<Q, F>(n);
    }

    /// Asserts that the number of events of the type `E` that were sent this frame is exactly `n`
    pub fn assert_n_events<E: Resource + PartialEq + Debug>(&self, n: usize) {
        self.world.assert_n_events::<E>(n);
    }

    /// Asserts that when the supplied `system` is run on the world, its output will be `true`
    ///
    /// WARNING: [`Changed`](crate::query::Changed) and [`Added`](crate::query::Added) filters are computed relative to "the last time this system ran".
    /// Because we are generating a new system; these filters will always be true.
    pub fn assert_system<Params>(&mut self, system: impl IntoSystem<(), bool, Params>) {
        self.world.assert_system(system);
    }
}
