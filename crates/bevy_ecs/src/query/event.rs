use super::{
    super::event::{EventReader, EventWriter, Events, ManualEventReader},
    Access, Fetch, FetchState, FilteredAccess, WorldQuery,
};
use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{Component, ComponentId},
    prelude::World,
    storage::{Table, Tables},
};
use std::marker::PhantomData;

impl<'a, T: Component> WorldQuery for EventWriter<'a, T> {
    type Fetch = EventWriterFetch<T>;
    type State = EventWriterState<T>;
}

struct EventWriterFetch<T> {
    marker: PhantomData<T>,
}

impl<'a, T: Component> Fetch<'a> for EventWriterFetch<T> {
    // EventWriter queries return an EventWriter<T> in each item
    type Item = EventWriter<'a, T>;
    // This is the corresponding S: FetchState type
    type State = EventWriterState<T>;

    unsafe fn init(
        world: &World,
        state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
    }

    fn is_dense(&self) -> bool {}

    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        archetype: &Archetype,
        tables: &Tables,
    ) {
    }

    unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {}

    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {}

    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {}
}
struct EventWriterState<T> {
    marker: PhantomData<T>,
}

unsafe impl<T: Component> FetchState for EventWriterState<T> {
    fn init(world: &mut World) -> Self {}

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {}

    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
    }

    fn matches_archetype(&self, archetype: &Archetype) -> bool {}

    fn matches_table(&self, table: &Table) -> bool {}
}
