use super::{
    super::event::{EventWriter, Events},
    Access, Fetch, FetchState, FilteredAccess, WorldQuery, WriteFetch, WriteState,
};
use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{Component, ComponentId, StorageType},
    prelude::World,
    storage::{Table, Tables},
};
use std::any::TypeId;

impl<'a, T: Component> WorldQuery for EventWriter<'a, T> {
    type Fetch = EventWriterFetch<'a, T>;
    type State = EventWriterState<T>;
}

struct EventWriterFetch<'s, T> {
    /// EventWriter query parameters require write access to &mut Events<T>
    write_fetch: WriteFetch<Events<T>>,
    state: &'s EventWriterState<T>,
}

impl<'a, T: Component> Fetch<'a> for EventWriterFetch<'a, T> {
    /// EventWriter queries return an EventWriter<T> in each item
    type Item = EventWriter<'a, T>;
    /// This is the corresponding S: FetchState type
    type State = EventWriterState<T>;

    /// Checks the storage type of the corresponding Events<T> component
    fn is_dense(&self) -> bool {
        match self.state.event_storage_type {
            StorageType::SparseSet => false,
            StorageType::Table => true,
        }
    }

    unsafe fn init(
        world: &World,
        state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        EventWriterFetch {
            write_fetch: WriteFetch::<Events<T>>::init(
                world,
                &state.write_state,
                last_change_tick,
                change_tick,
            ),
            state,
        }
    }

    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        archetype: &Archetype,
        tables: &Tables,
    ) {
        self.write_fetch
            .set_archetype(&state.write_state, archetype, tables);
    }

    unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {
        self.write_fetch.set_table(&state.write_state, table);
    }

    /// Returns the EventWriter<T> of the next entity when the storage type of the query is sparse
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        let events = *self.write_fetch.archetype_fetch(archetype_index);
        EventWriter {
            events: &mut events,
        }
    }

    /// Returns the EventWriter<T> of the next entity when the storage type of the query is dense
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        let events = *self.write_fetch.archetype_fetch(table_row);
        EventWriter {
            events: &mut events,
        }
    }
}
struct EventWriterState<T> {
    event_component_id: ComponentId,
    event_storage_type: StorageType,
    /// EventWriter query parameters require write access to &mut Events<T>
    write_state: WriteState<Events<T>>,
}

unsafe impl<T: Component> FetchState for EventWriterState<T> {
    fn init(world: &mut World) -> Self {
        let event_component_id = world.components.get_id(TypeId::of::<Events<T>>()).unwrap();
        EventWriterState {
            event_component_id,
            event_storage_type: world
                .components
                .get_info(event_component_id)
                .unwrap()
                .storage_type(),
            write_state: WriteState::<Events<T>>::init(world),
        }
    }

    /// Access is based on &Events<T>
    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        if access.access().has_write(self.event_component_id) {
            panic!("&{} conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                std::any::type_name::<Events<T>>());
        }
        access.add_read(self.event_component_id)
    }

    /// Access is based on &Events<T>
    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if let Some(archetype_component_id) =
            archetype.get_archetype_component_id(self.event_component_id)
        {
            access.add_read(archetype_component_id);
        }
    }

    /// Matches based on &Events<T>
    fn matches_archetype(&self, archetype: &Archetype) -> bool {
        archetype.contains(self.event_component_id)
    }

    /// Matches based on &Events<T>
    fn matches_table(&self, table: &Table) -> bool {
        table.has_column(self.event_component_id)
    }
}

// TODO: add tests
mod tests {}
