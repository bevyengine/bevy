use crate::Query;
use bevy_hecs::{
    impl_query_set, ArchetypeComponent, Fetch, Query as HecsQuery, QueryAccess, TypeAccess, World,
};

pub struct QuerySet<T: QueryTuple> {
    value: T,
}

impl_query_set!();

pub trait QueryTuple {
    /// # Safety
    /// this might cast world and component access to the relevant Self lifetimes. verify that this is safe in each impl
    unsafe fn new(world: &World, component_access: &TypeAccess<ArchetypeComponent>) -> Self;
    fn get_accesses() -> Vec<QueryAccess>;
}

impl<T: QueryTuple> QuerySet<T> {
    pub fn new(world: &World, component_access: &TypeAccess<ArchetypeComponent>) -> Self {
        QuerySet {
            value: unsafe { T::new(world, component_access) },
        }
    }
}
