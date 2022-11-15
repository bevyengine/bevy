//! Storage layouts for ECS data.

mod blob_vec;
mod resource;
mod sparse_set;
mod table;

pub use resource::*;
pub use sparse_set::*;
pub use table::*;

use crate::{
    archetype::Archetypes,
    component::{ComponentId, ComponentTicks, Components, StorageType, TickCells},
    entity::{Entity, EntityLocation},
};
use bevy_ptr::{OwningPtr, Ptr};
use std::any::TypeId;

/// The raw data stores of a [World](crate::world::World)
#[derive(Default)]
pub struct Storages {
    pub sparse_sets: SparseSets,
    pub tables: Tables,
    pub resources: Resources<true>,
    pub non_send_resources: Resources<false>,
}

impl Storages {
    /// Get a raw pointer to a particular [`Component`](crate::component::Component) and its [`ComponentTicks`] identified by their [`TypeId`]
    ///
    /// # Safety
    /// - `storage_type` must accurately reflect where the components for `component_id` are stored.
    /// - `location` must refer to an archetype that contains `entity`
    /// - `Archetypes` and `Components` must come from the world this of this `Storages`
    /// - the caller must ensure that no aliasing rules are violated
    #[inline]
    pub unsafe fn get_component_and_ticks_with_type(
        &self,
        archetypes: &Archetypes,
        components: &Components,
        type_id: TypeId,
        storage_type: StorageType,
        entity: Entity,
        location: EntityLocation,
    ) -> Option<(Ptr<'_>, TickCells<'_>)> {
        let component_id = components.get_id(type_id)?;
        // SAFETY: component_id is valid, the rest is deferred to caller
        self.get_component_and_ticks(archetypes, component_id, storage_type, entity, location)
    }

    /// Get a raw pointer to a particular [`Component`](crate::component::Component) and its [`ComponentTicks`]
    ///
    /// # Safety
    /// - `location` must refer to an archetype that contains `entity`
    /// - `component_id` must be valid
    /// - `storage_type` must accurately reflect where the components for `component_id` are stored.
    /// - `Archetypes` and `Components` must come from the world this of this `Storages`
    /// - the caller must ensure that no aliasing rules are violated
    #[inline]
    pub unsafe fn get_component_and_ticks(
        &self,
        archetypes: &Archetypes,
        component_id: ComponentId,
        storage_type: StorageType,
        entity: Entity,
        location: EntityLocation,
    ) -> Option<(Ptr<'_>, TickCells<'_>)> {
        match storage_type {
            StorageType::Table => {
                let (components, table_row) =
                    fetch_table(archetypes, self, location, component_id)?;

                // SAFETY: archetypes only store valid table_rows and the stored component type is T
                Some((
                    components.get_data_unchecked(table_row),
                    TickCells {
                        added: components.get_added_ticks_unchecked(table_row),
                        changed: components.get_changed_ticks_unchecked(table_row),
                    },
                ))
            }
            StorageType::SparseSet => fetch_sparse_set(self, component_id)?.get_with_ticks(entity),
        }
    }

    /// Get a raw pointer to a particular [`Component`](crate::component::Component) on a particular [`Entity`], identified by the component's type
    ///
    /// # Safety
    /// - `location` must refer to an archetype that contains `entity`
    /// the archetype
    /// - `storage_type` must accurately reflect where the components for `component_id` are stored.
    /// - `Archetypes` and `Components` must come from the world this of this `Storages`
    /// - the caller must ensure that no aliasing rules are violated
    #[inline]
    pub unsafe fn get_component_with_type(
        &self,
        archetypes: &Archetypes,
        components: &Components,
        type_id: TypeId,
        storage_type: StorageType,
        entity: Entity,
        location: EntityLocation,
    ) -> Option<Ptr<'_>> {
        let component_id = components.get_id(type_id)?;
        // SAFETY: component_id is valid, the rest is deferred to caller
        self.get_component(archetypes, component_id, storage_type, entity, location)
    }

    /// Get a raw pointer to a particular [`Component`](crate::component::Component) on a particular [`Entity`] in the provided [`World`](crate::world::World).
    ///
    /// # Safety
    /// - `location` must refer to an archetype that contains `entity`
    /// the archetype
    /// - `component_id`
    /// - `storage_type` must accurately reflect where the components for `component_id` are stored.
    /// - `Archetypes` and `Components` must come from the world this of this `Storages`
    /// - the caller must ensure that no aliasing rules are violated
    #[inline]
    pub unsafe fn get_component(
        &self,
        archetypes: &Archetypes,
        component_id: ComponentId,
        storage_type: StorageType,
        entity: Entity,
        location: EntityLocation,
    ) -> Option<Ptr<'_>> {
        // SAFETY: component_id exists and is therefore valid
        match storage_type {
            StorageType::Table => {
                let (components, table_row) =
                    fetch_table(archetypes, self, location, component_id)?;
                // SAFETY: archetypes only store valid table_rows and the stored component type is T
                Some(components.get_data_unchecked(table_row))
            }
            StorageType::SparseSet => fetch_sparse_set(self, component_id)?.get(entity),
        }
    }

    /// Get a raw pointer to the [`ComponentTicks`] on a particular [`Entity`], identified by the component's [`TypeId`]
    ///
    /// # Safety
    /// - `location` must refer to an archetype that contains `entity`
    /// the archetype
    /// - `storage_type` must accurately reflect where the components for `component_id` are stored.
    /// - `Archetypes` and `Components` must come from the world this of this `Storages`
    /// - the caller must ensure that no aliasing rules are violated
    #[inline]
    pub unsafe fn get_ticks_with_type(
        &self,
        archetypes: &Archetypes,
        components: &Components,
        type_id: TypeId,
        storage_type: StorageType,
        entity: Entity,
        location: EntityLocation,
    ) -> Option<ComponentTicks> {
        let component_id = components.get_id(type_id)?;
        // SAFETY: component_id is valid, the rest is deferred to caller
        self.get_ticks(archetypes, component_id, storage_type, entity, location)
    }

    /// Get a raw pointer to the [`ComponentTicks`] on a particular [`Entity`]
    ///
    /// # Safety
    /// - `location` must refer to an archetype that contains `entity`
    /// the archetype
    /// - `component_id` must be valid
    /// - `storage_type` must accurately reflect where the components for `component_id` are stored.
    /// - `Archetypes` and `Components` must come from the world this of this `Storages`
    /// - the caller must ensure that no aliasing rules are violated
    #[inline]
    pub unsafe fn get_ticks(
        &self,
        archetypes: &Archetypes,
        component_id: ComponentId,
        storage_type: StorageType,
        entity: Entity,
        location: EntityLocation,
    ) -> Option<ComponentTicks> {
        match storage_type {
            StorageType::Table => {
                let (components, table_row) =
                    fetch_table(archetypes, self, location, component_id)?;
                // SAFETY: archetypes only store valid table_rows and the stored component type is T
                Some(components.get_ticks_unchecked(table_row))
            }
            StorageType::SparseSet => fetch_sparse_set(self, component_id)?.get_ticks(entity),
        }
    }
}

impl Storages {
    /// Moves component data out of storage.
    ///
    /// This function leaves the underlying memory unchanged, but the component behind
    /// returned pointer is semantically owned by the caller and will not be dropped in its original location.
    /// Caller is responsible to drop component data behind returned pointer.
    ///
    /// # Safety
    /// - `location` must be within bounds of the given archetype and `entity` must exist inside the `archetype`
    /// - `component_id` must be valid
    /// - `components` must come from the same world as `self`
    /// - The relevant table row **must be removed** by the caller once all components are taken
    #[inline]
    pub(crate) unsafe fn take_component<'a>(
        &'a mut self,
        components: &Components,
        removed_components: &mut SparseSet<ComponentId, Vec<Entity>>,
        component_id: ComponentId,
        entity: Entity,
        location: EntityLocation,
    ) -> OwningPtr<'a> {
        let component_info = components.get_info_unchecked(component_id);
        let removed_components = removed_components.get_or_insert_with(component_id, Vec::new);
        removed_components.push(entity);
        match component_info.storage_type() {
            StorageType::Table => {
                let table = &mut self.tables[location.table_id];
                // SAFETY: archetypes will always point to valid columns
                let components = table.get_column_mut(component_id).unwrap();
                // SAFETY: archetypes only store valid table_rows and the stored component type is T
                components
                    .get_data_unchecked_mut(location.table_row)
                    .promote()
            }
            StorageType::SparseSet => self
                .sparse_sets
                .get_mut(component_id)
                .unwrap()
                .remove_and_forget(entity)
                .unwrap(),
        }
    }
}

#[inline]
unsafe fn fetch_table<'s>(
    archetypes: &Archetypes,
    storages: &'s Storages,
    location: EntityLocation,
    component_id: ComponentId,
) -> Option<(&'s Column, TableRow)> {
    let archetype = &archetypes[location.archetype_id];
    let table = &storages.tables[archetype.table_id()];
    let components = table.get_column(component_id)?;
    let table_row = archetype.entity_table_row(location.archetype_row);
    Some((components, table_row))
}

#[inline]
fn fetch_sparse_set(storages: &Storages, component_id: ComponentId) -> Option<&ComponentSparseSet> {
    storages.sparse_sets.get(component_id)
}
