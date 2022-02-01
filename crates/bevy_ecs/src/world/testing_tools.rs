//! Tools for convenient integration testing of the ECS.
//!
//! Each of these methods has a corresponding method on `App`;
//! in many cases, these are more convenient to use.
use crate::event::Events;
use crate::schedule::{Stage, SystemStage};
use crate::system::{In, IntoChainSystem, IntoSystem};
use crate::world::{FilterFetch, Mut, Resource, World, WorldQuery};
use std::fmt::Debug;

impl World {
    /// Asserts that that the current value of the resource `R` is `value`
    pub fn assert_resource_eq<R: Resource + PartialEq + Debug>(&self, value: R) {
        let resource = self
            .get_resource::<R>()
            .expect("No resource matching the type of {value} was found in the world.");
        assert_eq!(*resource, value);
    }

    /// Asserts that that the current value of the non-send resource `NS` is `value`
    pub fn assert_nonsend_resource_eq<NS: 'static + PartialEq + Debug>(&self, value: NS) {
        let resource = self
            .get_non_send_resource::<NS>()
            .expect("No non-send resource matching the type of {value} was found in the world.");
        assert_eq!(*resource, value);
    }

    /// Asserts that the number of entities returned by the query is exactly `n`
    pub fn assert_n_in_query<Q, F>(&mut self, n: usize)
    where
        Q: WorldQuery,
        F: WorldQuery,
        <F as WorldQuery>::Fetch: FilterFetch,
    {
        let mut query_state = self.query_filtered::<Q, F>();
        assert_eq!(query_state.iter(self).count(), n);
    }

    /// Sends an `event` of type `E`
    pub fn send_event<E: Resource>(&mut self, event: E) {
        let mut events: Mut<Events<E>> = self.get_resource_mut()
        .expect("The specified event resource was not found in the world. Did you forget to call `app.add_event::<E>()`?");

        events.send(event);
    }

    /// Asserts that the number of events of the type `E` that were sent this frame is exactly `n`
    pub fn assert_n_events<E: Resource + Debug>(&self, n: usize) {
        let events = self.get_resource::<Events<E>>().unwrap();

        assert_eq!(events.iter_current_update_events().count(), n);
    }

    /// Asserts that when the supplied `system` is run on the world, its output will be `true`
    ///
    /// WARNING: [`Changed`](crate::query::Changed) and [`Added`](crate::query::Added) filters are computed relative to "the last time this system ran".
    /// Because we are generating a new system; these filters will always be true.
    pub fn assert_system<T: 'static, E: 'static, Params>(
        &mut self,
        system: impl IntoSystem<(), Result<T, E>, Params>,
    ) {
        let mut stage = SystemStage::single_threaded();
        stage.add_system(system.chain(assert_system_input_true));
        stage.run(self);
    }
}

/// A chainable system that panics if its `input` is not okay
fn assert_system_input_true<T, E>(In(result): In<Result<T, E>>) {
    assert!(result.is_ok());
}
