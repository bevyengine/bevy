//! Tools for convenient integration testing of the ECS.
//!
//! Each of these methods has a corresponding method on `App`;
//! in many cases, these are more convenient to use.

use crate::component::Component;
use crate::entity::Entity;
use crate::world::{FilterFetch, World, WorldQuery};
use std::fmt::Debug;

impl World {
    /// Asserts that all components of type `C` returned by a query with the filter `F` will equal `value`
    ///
    /// This is commonly used with the corresponding `query_len` method to ensure that the returned query is not empty.
    ///
    /// WARNING: because we are constructing the query from scratch,
    /// [`Changed`](crate::query::Changed) and [`Added`](crate::query::Added) filters
    /// will always return true.
    pub fn assert_component_eq<C, F>(&mut self, value: &C)
    where
        C: Component + PartialEq + Debug,
        F: WorldQuery,
        <F as WorldQuery>::Fetch: FilterFetch,
    {
        let mut query_state = self.query_filtered::<(Entity, &C), F>();
        for (entity, component) in query_state.iter(self) {
            if component != value {
                panic!(
                    "Found component {component:?} for {entity:?}, but was expecting {value:?}."
                );
            }
        }
    }
}
