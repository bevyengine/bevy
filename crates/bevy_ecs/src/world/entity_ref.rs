use crate::{
    archetype::{Archetype, ArchetypeId, Archetypes},
    bundle::{Bundle, BundleInfo},
    change_detection::Ticks,
    component::{Component, ComponentId, ComponentTicks, Components, StorageType},
    entity::{Entities, Entity, EntityLocation},
    storage::{SparseSet, Storages},
    world::{Mut, World},
};
use std::any::TypeId;

pub struct EntityRef<'w> {
    world: &'w World,
    entity: Entity,
    location: EntityLocation,
}

impl<'w> EntityRef<'w> {
    #[inline]
    pub(crate) fn new(world: &'w World, entity: Entity, location: EntityLocation) -> Self {
        Self {
            world,
            entity,
            location,
        }
    }

    #[inline]
    pub fn id(&self) -> Entity {
        self.entity
    }

    #[inline]
    pub fn location(&self) -> EntityLocation {
        self.location
    }

    #[inline]
    pub fn archetype(&self) -> &Archetype {
        &self.world.archetypes[self.location.archetype_id]
    }

    #[inline]
    pub fn world(&mut self) -> &World {
        self.world
    }

    #[inline]
    pub fn contains<T: Component>(&self) -> bool {
        self.contains_type_id(TypeId::of::<T>())
    }

    #[inline]
    pub fn contains_id(&self, component_id: ComponentId) -> bool {
        contains_component_with_id(self.world, component_id, self.location)
    }

    #[inline]
    pub fn contains_type_id(&self, type_id: TypeId) -> bool {
        contains_component_with_type(self.world, type_id, self.location)
    }

    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'w T> {
        // SAFE: entity location is valid and returned component is of type T
        unsafe {
            get_component_with_type(self.world, TypeId::of::<T>(), self.entity, self.location)
                .map(|value| &*value.cast::<T>())
        }
    }

    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple
    /// mutable references to the same component
    #[inline]
    pub unsafe fn get_unchecked_mut<T: Component>(
        &self,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Option<Mut<'w, T>> {
        get_component_and_ticks_with_type(self.world, TypeId::of::<T>(), self.entity, self.location)
            .map(|(value, ticks)| Mut {
                value: &mut *value.cast::<T>(),
                ticks: Ticks {
                    component_ticks: &mut *ticks,
                    last_change_tick,
                    change_tick,
                },
            })
    }
}

pub struct EntityMut<'w> {
    world: &'w mut World,
    entity: Entity,
    location: EntityLocation,
}

impl<'w> EntityMut<'w> {
    /// # Safety
    /// entity and location _must_ be valid
    #[inline]
    pub(crate) unsafe fn new(
        world: &'w mut World,
        entity: Entity,
        location: EntityLocation,
    ) -> Self {
        EntityMut {
            world,
            entity,
            location,
        }
    }

    #[inline]
    pub fn id(&self) -> Entity {
        self.entity
    }

    #[inline]
    pub fn location(&self) -> EntityLocation {
        self.location
    }

    #[inline]
    pub fn archetype(&self) -> &Archetype {
        &self.world.archetypes[self.location.archetype_id]
    }

    #[inline]
    pub fn contains<T: Component>(&self) -> bool {
        self.contains_type_id(TypeId::of::<T>())
    }

    #[inline]
    pub fn contains_id(&self, component_id: ComponentId) -> bool {
        contains_component_with_id(self.world, component_id, self.location)
    }

    #[inline]
    pub fn contains_type_id(&self, type_id: TypeId) -> bool {
        contains_component_with_type(self.world, type_id, self.location)
    }

    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'w T> {
        // SAFE: entity location is valid and returned component is of type T
        unsafe {
            get_component_with_type(self.world, TypeId::of::<T>(), self.entity, self.location)
                .map(|value| &*value.cast::<T>())
        }
    }

    #[inline]
    pub fn get_mut<T: Component>(&mut self) -> Option<Mut<'w, T>> {
        // SAFE: world access is unique, entity location is valid, and returned component is of type
        // T
        unsafe {
            get_component_and_ticks_with_type(
                self.world,
                TypeId::of::<T>(),
                self.entity,
                self.location,
            )
            .map(|(value, ticks)| Mut {
                value: &mut *value.cast::<T>(),
                ticks: Ticks {
                    component_ticks: &mut *ticks,
                    last_change_tick: self.world.last_change_tick(),
                    change_tick: self.world.change_tick(),
                },
            })
        }
    }

    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple
    /// mutable references to the same component
    #[inline]
    pub unsafe fn get_unchecked_mut<T: Component>(&self) -> Option<Mut<'w, T>> {
        get_component_and_ticks_with_type(self.world, TypeId::of::<T>(), self.entity, self.location)
            .map(|(value, ticks)| Mut {
                value: &mut *value.cast::<T>(),
                ticks: Ticks {
                    component_ticks: &mut *ticks,
                    last_change_tick: self.world.last_change_tick(),
                    change_tick: self.world.read_change_tick(),
                },
            })
    }

    pub fn insert_bundle<T: Bundle>(&mut self, bundle: T) -> &mut Self {
        let change_tick = self.world.change_tick();
        let bundle_info = self
            .world
            .bundles
            .init_info::<T>(&mut self.world.components, &mut self.world.storages);
        let mut bundle_inserter = bundle_info.get_bundle_inserter(
            &mut self.world.entities,
            &mut self.world.archetypes,
            &mut self.world.components,
            &mut self.world.storages,
            self.location.archetype_id,
            change_tick,
        );
        // SAFE: location matches current entity. `T` matches `bundle_info`
        unsafe {
            self.location = bundle_inserter.insert(self.entity, self.location.index, bundle);
        }

        self
    }

    // TODO: move to BundleInfo
    pub fn remove_bundle<T: Bundle>(&mut self) -> Option<T> {
        let archetypes = &mut self.world.archetypes;
        let storages = &mut self.world.storages;
        let components = &mut self.world.components;
        let entities = &mut self.world.entities;
        let removed_components = &mut self.world.removed_components;

        let bundle_info = self.world.bundles.init_info::<T>(components, storages);
        let old_location = self.location;
        let new_archetype_id = unsafe {
            remove_bundle_from_archetype(
                archetypes,
                storages,
                components,
                old_location.archetype_id,
                bundle_info,
                false,
            )?
        };

        if new_archetype_id == old_location.archetype_id {
            return None;
        }

        let old_archetype = &mut archetypes[old_location.archetype_id];
        let mut bundle_components = bundle_info.component_ids.iter().cloned();
        let entity = self.entity;
        // SAFE: bundle components are iterated in order, which guarantees that the component type
        // matches
        let result = unsafe {
            T::from_components(|| {
                let component_id = bundle_components.next().unwrap();
                // SAFE: entity location is valid and table row is removed below
                take_component(
                    components,
                    storages,
                    old_archetype,
                    removed_components,
                    component_id,
                    entity,
                    old_location,
                )
            })
        };

        unsafe {
            Self::move_entity_from_remove::<false>(
                entity,
                &mut self.location,
                old_location.archetype_id,
                old_location,
                entities,
                archetypes,
                storages,
                new_archetype_id,
            );
        }

        Some(result)
    }

    /// Safety: `new_archetype_id` must have the same or a subset of the components
    /// in `old_archetype_id`. Probably more safety stuff too, audit a call to
    /// this fn as if the code here was written inline
    ///
    /// when DROP is true removed components will be dropped otherwise they will be forgotten
    ///
    // We use a const generic here so that we are less reliant on
    // inlining for rustc to optimize out the `match DROP`
    #[allow(clippy::too_many_arguments)]
    unsafe fn move_entity_from_remove<const DROP: bool>(
        entity: Entity,
        self_location: &mut EntityLocation,
        old_archetype_id: ArchetypeId,
        old_location: EntityLocation,
        entities: &mut Entities,
        archetypes: &mut Archetypes,
        storages: &mut Storages,
        new_archetype_id: ArchetypeId,
    ) {
        let old_archetype = &mut archetypes[old_archetype_id];
        let remove_result = old_archetype.swap_remove(old_location.index);
        if let Some(swapped_entity) = remove_result.swapped_entity {
            entities.meta[swapped_entity.id as usize].location = old_location;
        }
        let old_table_row = remove_result.table_row;
        let old_table_id = old_archetype.table_id();
        let new_archetype = &mut archetypes[new_archetype_id];

        let new_location = if old_table_id == new_archetype.table_id() {
            new_archetype.allocate(entity, old_table_row)
        } else {
            let (old_table, new_table) = storages
                .tables
                .get_2_mut(old_table_id, new_archetype.table_id());

            // SAFE: old_table_row exists
            let move_result = if DROP {
                old_table.move_to_and_drop_missing_unchecked(old_table_row, new_table)
            } else {
                old_table.move_to_and_forget_missing_unchecked(old_table_row, new_table)
            };

            // SAFE: move_result.new_row is a valid position in new_archetype's table
            let new_location = new_archetype.allocate(entity, move_result.new_row);

            // if an entity was moved into this entity's table spot, update its table row
            if let Some(swapped_entity) = move_result.swapped_entity {
                let swapped_location = entities.get(swapped_entity).unwrap();
                archetypes[swapped_location.archetype_id]
                    .set_entity_table_row(swapped_location.index, old_table_row);
            }

            new_location
        };

        *self_location = new_location;
        entities.meta[entity.id as usize].location = new_location;
    }

    // TODO: move to BundleInfo
    /// Remove any components in the bundle that the entity has.
    pub fn remove_bundle_intersection<T: Bundle>(&mut self) {
        let archetypes = &mut self.world.archetypes;
        let storages = &mut self.world.storages;
        let components = &mut self.world.components;
        let entities = &mut self.world.entities;
        let removed_components = &mut self.world.removed_components;

        let bundle_info = self.world.bundles.init_info::<T>(components, storages);
        let old_location = self.location;
        let new_archetype_id = unsafe {
            remove_bundle_from_archetype(
                archetypes,
                storages,
                components,
                old_location.archetype_id,
                bundle_info,
                true,
            )
            .expect("intersections should always return a result")
        };

        if new_archetype_id == old_location.archetype_id {
            return;
        }

        let old_archetype = &mut archetypes[old_location.archetype_id];
        let entity = self.entity;
        for component_id in bundle_info.component_ids.iter().cloned() {
            if old_archetype.contains(component_id) {
                removed_components
                    .get_or_insert_with(component_id, Vec::new)
                    .push(entity);

                // Make sure to drop components stored in sparse sets.
                // Dense components are dropped later in `move_to_and_drop_missing_unchecked`.
                if let Some(StorageType::SparseSet) = old_archetype.get_storage_type(component_id) {
                    storages
                        .sparse_sets
                        .get_mut(component_id)
                        .unwrap()
                        .remove(entity);
                }
            }
        }

        unsafe {
            Self::move_entity_from_remove::<true>(
                entity,
                &mut self.location,
                old_location.archetype_id,
                old_location,
                entities,
                archetypes,
                storages,
                new_archetype_id,
            )
        }
    }

    pub fn insert<T: Component>(&mut self, value: T) -> &mut Self {
        self.insert_bundle((value,))
    }

    pub fn remove<T: Component>(&mut self) -> Option<T> {
        self.remove_bundle::<(T,)>().map(|v| v.0)
    }

    pub fn despawn(self) {
        let world = self.world;
        world.flush();
        let location = world
            .entities
            .free(self.entity)
            .expect("entity should exist at this point.");
        let table_row;
        let moved_entity;
        {
            let archetype = &mut world.archetypes[location.archetype_id];
            for component_id in archetype.components() {
                let removed_components = world
                    .removed_components
                    .get_or_insert_with(component_id, Vec::new);
                removed_components.push(self.entity);
            }
            let remove_result = archetype.swap_remove(location.index);
            if let Some(swapped_entity) = remove_result.swapped_entity {
                world.entities.meta[swapped_entity.id as usize].location = location;
            }
            table_row = remove_result.table_row;

            for component_id in archetype.sparse_set_components() {
                let sparse_set = world.storages.sparse_sets.get_mut(*component_id).unwrap();
                sparse_set.remove(self.entity);
            }
            // SAFE: table rows stored in archetypes always exist
            moved_entity = unsafe {
                world.storages.tables[archetype.table_id()].swap_remove_unchecked(table_row)
            };
        };

        if let Some(moved_entity) = moved_entity {
            let moved_location = world.entities.get(moved_entity).unwrap();
            world.archetypes[moved_location.archetype_id]
                .set_entity_table_row(moved_location.index, table_row);
        }
    }

    #[inline]
    pub fn world(&mut self) -> &World {
        self.world
    }

    /// # Safety
    /// Caller must not modify the world in a way that changes the current entity's location
    /// If the caller _does_ do something that could change the location, self.update_location()
    /// must be called before using any other methods in EntityMut
    #[inline]
    pub unsafe fn world_mut(&mut self) -> &mut World {
        self.world
    }

    /// Updates the internal entity location to match the current location in the internal [World].
    /// This is only needed if the user called [EntityMut::world], which enables the location to
    /// change.
    pub fn update_location(&mut self) {
        self.location = self.world.entities().get(self.entity).unwrap();
    }
}

// TODO: move to Storages?
/// # Safety
/// `entity_location` must be within bounds of the given archetype and `entity` must exist inside
/// the archetype
#[inline]
unsafe fn get_component(
    world: &World,
    component_id: ComponentId,
    entity: Entity,
    location: EntityLocation,
) -> Option<*mut u8> {
    let archetype = &world.archetypes[location.archetype_id];
    // SAFE: component_id exists and is therefore valid
    let component_info = world.components.get_info_unchecked(component_id);
    match component_info.storage_type() {
        StorageType::Table => {
            let table = &world.storages.tables[archetype.table_id()];
            let components = table.get_column(component_id)?;
            let table_row = archetype.entity_table_row(location.index);
            // SAFE: archetypes only store valid table_rows and the stored component type is T
            Some(components.get_data_unchecked(table_row))
        }
        StorageType::SparseSet => world
            .storages
            .sparse_sets
            .get(component_id)
            .and_then(|sparse_set| sparse_set.get(entity)),
    }
}

// TODO: move to Storages?
/// # Safety
/// Caller must ensure that `component_id` is valid
#[inline]
unsafe fn get_component_and_ticks(
    world: &World,
    component_id: ComponentId,
    entity: Entity,
    location: EntityLocation,
) -> Option<(*mut u8, *mut ComponentTicks)> {
    let archetype = &world.archetypes[location.archetype_id];
    let component_info = world.components.get_info_unchecked(component_id);
    match component_info.storage_type() {
        StorageType::Table => {
            let table = &world.storages.tables[archetype.table_id()];
            let components = table.get_column(component_id)?;
            let table_row = archetype.entity_table_row(location.index);
            // SAFE: archetypes only store valid table_rows and the stored component type is T
            Some((
                components.get_data_unchecked(table_row),
                components.get_ticks_mut_ptr_unchecked(table_row),
            ))
        }
        StorageType::SparseSet => world
            .storages
            .sparse_sets
            .get(component_id)
            .and_then(|sparse_set| sparse_set.get_with_ticks(entity)),
    }
}

// TODO: move to Storages?
/// Moves component data out of storage.
///
/// This function leaves the underlying memory unchanged, but the component behind
/// returned pointer is semantically owned by the caller and will not be dropped in its original location.
/// Caller is responsible to drop component data behind returned pointer.
///
/// # Safety
/// - `entity_location` must be within bounds of the given archetype and `entity` must exist inside the archetype
/// - `component_id` must be valid
/// - The relevant table row **must be removed** by the caller once all components are taken
#[inline]
unsafe fn take_component(
    components: &Components,
    storages: &mut Storages,
    archetype: &Archetype,
    removed_components: &mut SparseSet<ComponentId, Vec<Entity>>,
    component_id: ComponentId,
    entity: Entity,
    location: EntityLocation,
) -> *mut u8 {
    let component_info = components.get_info_unchecked(component_id);
    let removed_components = removed_components.get_or_insert_with(component_id, Vec::new);
    removed_components.push(entity);
    match component_info.storage_type() {
        StorageType::Table => {
            let table = &storages.tables[archetype.table_id()];
            // SAFE: archetypes will always point to valid columns
            let components = table.get_column(component_id).unwrap();
            let table_row = archetype.entity_table_row(location.index);
            // SAFE: archetypes only store valid table_rows and the stored component type is T
            components.get_data_unchecked(table_row)
        }
        StorageType::SparseSet => storages
            .sparse_sets
            .get_mut(component_id)
            .unwrap()
            .remove_and_forget(entity)
            .unwrap(),
    }
}

/// # Safety
/// `entity_location` must be within bounds of an archetype that exists.
unsafe fn get_component_with_type(
    world: &World,
    type_id: TypeId,
    entity: Entity,
    location: EntityLocation,
) -> Option<*mut u8> {
    let component_id = world.components.get_id(type_id)?;
    get_component(world, component_id, entity, location)
}

/// # Safety
/// `entity_location` must be within bounds of an archetype that exists.
pub(crate) unsafe fn get_component_and_ticks_with_type(
    world: &World,
    type_id: TypeId,
    entity: Entity,
    location: EntityLocation,
) -> Option<(*mut u8, *mut ComponentTicks)> {
    let component_id = world.components.get_id(type_id)?;
    get_component_and_ticks(world, component_id, entity, location)
}

fn contains_component_with_type(world: &World, type_id: TypeId, location: EntityLocation) -> bool {
    if let Some(component_id) = world.components.get_id(type_id) {
        contains_component_with_id(world, component_id, location)
    } else {
        false
    }
}

fn contains_component_with_id(
    world: &World,
    component_id: ComponentId,
    location: EntityLocation,
) -> bool {
    world.archetypes[location.archetype_id].contains(component_id)
}

/// Removes a bundle from the given archetype and returns the resulting archetype (or None if the
/// removal was invalid). in the event that adding the given bundle does not result in an Archetype
/// change. Results are cached in the Archetype Graph to avoid redundant work.
/// if `intersection` is false, attempting to remove a bundle with components _not_ contained in the
/// current archetype will fail, returning None. if `intersection` is true, components in the bundle
/// but not in the current archetype will be ignored
///
/// # Safety
/// `archetype_id` must exist and components in `bundle_info` must exist
unsafe fn remove_bundle_from_archetype(
    archetypes: &mut Archetypes,
    storages: &mut Storages,
    components: &mut Components,
    archetype_id: ArchetypeId,
    bundle_info: &BundleInfo,
    intersection: bool,
) -> Option<ArchetypeId> {
    // check the archetype graph to see if the Bundle has been removed from this archetype in the
    // past
    let remove_bundle_result = {
        let current_archetype = &mut archetypes[archetype_id];
        if intersection {
            current_archetype
                .edges()
                .get_remove_bundle_intersection(bundle_info.id)
        } else {
            current_archetype.edges().get_remove_bundle(bundle_info.id)
        }
    };
    let result = if let Some(result) = remove_bundle_result {
        // this Bundle removal result is cached. just return that!
        result
    } else {
        let mut next_table_components;
        let mut next_sparse_set_components;
        let next_table_id;
        {
            let current_archetype = &mut archetypes[archetype_id];
            let mut removed_table_components = Vec::new();
            let mut removed_sparse_set_components = Vec::new();
            for component_id in bundle_info.component_ids.iter().cloned() {
                if current_archetype.contains(component_id) {
                    // SAFE: bundle components were already initialized by bundles.get_info
                    let component_info = components.get_info_unchecked(component_id);
                    match component_info.storage_type() {
                        StorageType::Table => removed_table_components.push(component_id),
                        StorageType::SparseSet => removed_sparse_set_components.push(component_id),
                    }
                } else if !intersection {
                    // a component in the bundle was not present in the entity's archetype, so this
                    // removal is invalid cache the result in the archetype
                    // graph
                    current_archetype
                        .edges_mut()
                        .insert_remove_bundle(bundle_info.id, None);
                    return None;
                }
            }

            // sort removed components so we can do an efficient "sorted remove". archetype
            // components are already sorted
            removed_table_components.sort();
            removed_sparse_set_components.sort();
            next_table_components = current_archetype.table_components().to_vec();
            next_sparse_set_components = current_archetype.sparse_set_components().to_vec();
            sorted_remove(&mut next_table_components, &removed_table_components);
            sorted_remove(
                &mut next_sparse_set_components,
                &removed_sparse_set_components,
            );

            next_table_id = if removed_table_components.is_empty() {
                current_archetype.table_id()
            } else {
                // SAFE: all components in next_table_components exist
                storages
                    .tables
                    .get_id_or_insert(&next_table_components, components)
            };
        }

        let new_archetype_id = archetypes.get_id_or_insert(
            next_table_id,
            next_table_components,
            next_sparse_set_components,
        );
        Some(new_archetype_id)
    };
    let current_archetype = &mut archetypes[archetype_id];
    // cache the result in an edge
    if intersection {
        current_archetype
            .edges_mut()
            .insert_remove_bundle_intersection(bundle_info.id, result);
    } else {
        current_archetype
            .edges_mut()
            .insert_remove_bundle(bundle_info.id, result);
    }
    result
}

fn sorted_remove<T: Eq + Ord + Copy>(source: &mut Vec<T>, remove: &[T]) {
    let mut remove_index = 0;
    source.retain(|value| {
        while remove_index < remove.len() && *value > remove[remove_index] {
            remove_index += 1;
        }

        if remove_index < remove.len() {
            *value != remove[remove_index]
        } else {
            true
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn sorted_remove() {
        let mut a = vec![1, 2, 3, 4, 5, 6, 7];
        let b = vec![1, 2, 3, 5, 7];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![4, 6]);

        let mut a = vec![1];
        let b = vec![1];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![]);

        let mut a = vec![1];
        let b = vec![2];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![1]);
    }
}
