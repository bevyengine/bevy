use crate::event::Events;
use crate::query::{Fetch, WorldQuery};
use crate::schedule::{Stage, SystemStage};
use crate::system::{In, IntoChainSystem, IntoSystem};
use crate::world::{FilterFetch, Resource, World};
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

    /// Asserts that each item returned by the provided query is equal to the provided `value`
    pub fn assert_query_items_eq<'w, 's1, 's2, Q, F>(
        // Reference to the world must live at least as long as the query state
        // FIXME: first lifetime points to lifetime on &mut self
        &'s2 mut self,
        // Reference to the value must live at least as long as the query state
        // FIXME: Second lifetime points to lifetime on Item
        value: &'s2 <Q::ReadOnlyFetch as Fetch<'w, 's2>>::Item,
    ) where
        Q: WorldQuery,
        F: WorldQuery,
        <Q::ReadOnlyFetch as Fetch<'w, 's1>>::Item: PartialEq + Debug,
        <F as WorldQuery>::Fetch: FilterFetch,
        // World must outlive query state
        'w: 's1,
        'w: 's2,
    {
        let mut query_state = self.query_filtered::<Q, F>();
        for item in query_state.iter(self) {
            // item has lifetimes 'w, 's1
            // value has lifetimes 'w, 's2
            // FIXME: the lifetime `'s1` does not necessarily outlive the lifetime `'s2`
            assert_eq!(item, *value);
        }
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

    /// Asserts that the number of events of the type `E` that were sent this frame is exactly `n`
    pub fn assert_n_events<E: Resource + PartialEq + Debug>(&self, n: usize) {
        let events = self.get_resource::<Events<E>>().unwrap();

        assert_eq!(events.iter_current_update_events().count(), n);
    }

    /// Asserts that when the supplied `system` is run on the world, its output will be `true`
    pub fn assert_system<Params>(&mut self, system: impl IntoSystem<(), bool, Params>) {
        let mut stage = SystemStage::single_threaded();
        stage.add_system(system.chain(assert_system_input_true));
        stage.run(self);
    }
}

/// A chainable system that panics if its `input` is not `true`
fn assert_system_input_true(In(result): In<bool>) {
    assert!(result);
}
