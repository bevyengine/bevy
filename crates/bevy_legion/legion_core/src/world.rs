use crate::borrow::Ref;
use crate::borrow::RefMut;
use crate::entity::BlockAllocator;
use crate::entity::Entity;
use crate::entity::EntityLocation;
use crate::entity::Locations;
use crate::event::Event;
use crate::filter::ArchetypeFilterData;
use crate::filter::ChunksetFilterData;
use crate::filter::EntityFilter;
use crate::filter::Filter;
use crate::guid_entity_allocator::GuidEntityAllocator;
use crate::index::ArchetypeIndex;
use crate::index::ComponentIndex;
use crate::index::SetIndex;
use crate::iterator::SliceVecIter;
use crate::storage::ArchetypeData;
use crate::storage::ArchetypeDescription;
use crate::storage::Component;
use crate::storage::ComponentMeta;
use crate::storage::ComponentStorage;
use crate::storage::ComponentTypeId;
use crate::storage::Storage;
use crate::storage::Tag;
use crate::storage::TagMeta;
use crate::storage::TagTypeId;
use crate::storage::Tags;
use crate::tuple::TupleEq;
use parking_lot::Mutex;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::iter::Enumerate;
use std::iter::Fuse;
use std::iter::FusedIterator;
use std::iter::Peekable;
use std::iter::Repeat;
use std::iter::Take;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, span, trace, Level};

static NEXT_UNIVERSE_ID: AtomicUsize = AtomicUsize::new(1);
static NEXT_WORLD_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UniverseId(usize);

/// The `Universe` is a factory for creating `World`s.
///
/// Entities inserted into worlds created within the same universe are guarenteed to have
/// unique `Entity` IDs, even across worlds.
#[derive(Debug)]
pub struct Universe {
    id: UniverseId,
    allocator: Arc<Mutex<BlockAllocator>>,
}

impl Universe {
    /// Creates a new `Universe`.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            id: UniverseId(NEXT_UNIVERSE_ID.fetch_add(1, Ordering::SeqCst)),
            allocator: Arc::new(Mutex::new(BlockAllocator::new())),
        }
    }

    /// Creates a new `World` within this `Universe`.
    ///
    /// Entities inserted into worlds created within the same universe are guarenteed to have
    /// unique `Entity` IDs, even across worlds. See also `World::new`.
    pub fn create_world(&self) -> World {
        let id = WorldId::next(self.id.0);
        let world = World::new_in_universe(id, GuidEntityAllocator::default());

        info!(universe = self.id.0, world = world.id().1, "Created world");
        world
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct WorldId(usize, usize);

impl WorldId {
    fn next(universe: usize) -> Self {
        Self(universe, NEXT_WORLD_ID.fetch_add(1, Ordering::SeqCst))
    }

    pub fn index(self) -> usize { self.0 }

    pub fn is_same_universe(self, other: WorldId) -> bool { self.0 == other.0 }
}

/// Contains queryable collections of data associated with `Entity`s.
pub struct World {
    id: WorldId,
    storage: UnsafeCell<Storage>,
    pub entity_allocator: Arc<GuidEntityAllocator>,
    entity_locations: Locations,
    defrag_progress: usize,
    command_buffer_size: usize,
    pub(crate) allocation_buffer: Vec<Entity>,
}

unsafe impl Send for World {}

unsafe impl Sync for World {}

impl World {
    pub const DEFAULT_COMMAND_BUFFER_SIZE: usize = 64;

    /// Create a new `World` independent of any `Universe`.
    ///
    /// `Entity` IDs in such a world will only be unique within that world. See also
    /// `Universe::create_world`.
    pub fn new() -> Self { Self::new_in_universe(WorldId::next(0), GuidEntityAllocator::default()) }

    fn new_in_universe(id: WorldId, allocator: GuidEntityAllocator) -> Self {
        Self {
            id,
            storage: UnsafeCell::new(Storage::new(id)),
            entity_allocator: Arc::new(allocator),
            entity_locations: Locations::new(),
            defrag_progress: 0,
            command_buffer_size: Self::DEFAULT_COMMAND_BUFFER_SIZE,
            allocation_buffer: Vec::with_capacity(Self::DEFAULT_COMMAND_BUFFER_SIZE),
        }
    }

    #[inline]
    pub fn command_buffer_size(&self) -> usize { self.command_buffer_size }

    #[inline]
    pub fn set_command_buffer_size(&mut self, command_buffer_size: usize) {
        self.command_buffer_size = command_buffer_size;
    }

    /// Subscribes to event notifications.
    ///
    /// A filter determines which events are of interest. Use `any()` to listen to all events.
    ///
    /// # Examples
    ///
    /// ```
    /// # use legion_core::prelude::*;
    /// # #[derive(Copy, Clone, Debug, PartialEq)]
    /// # struct Position(f32);
    /// # #[derive(Copy, Clone, Debug, PartialEq)]
    /// # struct Model;
    /// # let universe = Universe::new();
    /// # let mut world = universe.create_world();
    /// let (sender, receiver) = crossbeam_channel::unbounded();
    /// world.subscribe(sender, component::<Position>() | tag::<Model>());
    ///
    /// for event in receiver.try_iter() {
    ///     println!("{:?}", event);
    /// }
    /// ```
    pub fn subscribe<T: EntityFilter + Sync + 'static>(
        &mut self,
        sender: crossbeam_channel::Sender<Event>,
        filter: T,
    ) {
        self.storage_mut().subscribe(sender, filter);
    }

    pub fn storage(&self) -> &Storage { unsafe { &*self.storage.get() } }

    pub fn storage_mut(&mut self) -> &mut Storage { unsafe { &mut *self.storage.get() } }

    /// Gets the unique ID of this world within its universe.
    pub fn id(&self) -> WorldId { self.id }

    pub fn get_entity_location(&self, entity: Entity) -> Option<EntityLocation> {
        if self.is_alive(entity) {
            self.entity_locations.get(entity)
        } else {
            None
        }
    }

    /// Iterate all entities in existence. Internally this iterates archetypes instead of
    /// entity allocators because the data structures contains a list of free entities instead
    /// of allocated entities
    pub fn iter_entities<'a>(&'a self) -> impl Iterator<Item = Entity> + 'a {
        self.storage()
            .archetypes()
            .iter()
            .flat_map(|archetype_data| archetype_data.iter_entities().map(|entity| entity))
    }

    /// Inserts new entities into the world. This insertion method should be preferred, as it performs
    /// no movement of components for inserting multiple entities and components.
    ///
    /// # Examples
    ///
    /// Inserting entity tuples:
    ///
    /// ```
    /// # use legion_core::prelude::*;
    /// # #[derive(Copy, Clone, Debug, PartialEq)]
    /// # struct Position(f32);
    /// # #[derive(Copy, Clone, Debug, PartialEq)]
    /// # struct Rotation(f32);
    /// # let universe = Universe::new();
    /// # let mut world = universe.create_world();
    /// # let model = 0u8;
    /// # let color = 0u16;
    /// let tags = (model, color);
    /// let data = vec![
    ///     (Position(0.0), Rotation(0.0)),
    ///     (Position(1.0), Rotation(1.0)),
    ///     (Position(2.0), Rotation(2.0)),
    /// ];
    /// world.insert(tags, data);
    /// ```
    #[inline]
    pub fn insert<T, C>(&mut self, tags: T, components: C) -> &[Entity]
    where
        T: TagSet + TagLayout + for<'a> Filter<ChunksetFilterData<'a>>,
        C: IntoComponentSource,
    {
        self.insert_impl(tags, components.into())
    }

    pub(crate) fn insert_impl<T, C>(&mut self, mut tags: T, mut components: C) -> &[Entity]
    where
        T: TagSet + TagLayout + for<'a> Filter<ChunksetFilterData<'a>>,
        C: ComponentSource,
    {
        let span = span!(Level::TRACE, "Inserting entities", world = self.id().0);
        let _guard = span.enter();

        // find or create archetype
        let archetype_index = self.find_or_create_archetype(&mut tags, &mut components);

        // find or create chunk set
        let chunk_set_index = self.find_or_create_chunk(archetype_index, &mut tags);

        self.allocation_buffer.clear();
        self.allocation_buffer.reserve(components.len());

        // insert components into chunks
        while !components.is_empty() {
            // get chunk component storage
            let archetype =
                unsafe { (&mut *self.storage.get()).archetype_unchecked_mut(archetype_index) };
            let chunk_index = archetype.get_free_chunk(chunk_set_index, 1);
            let chunk = unsafe {
                archetype
                    .chunkset_unchecked_mut(chunk_set_index)
                    .chunk_unchecked_mut(chunk_index)
            };

            // insert as many components as we can into the chunk
            let allocated = components.write(self.entity_allocator.create_entities(), chunk);

            // record new entity locations
            let start = chunk.len() - allocated;
            let added = chunk.entities().iter().enumerate().skip(start);
            for (i, e) in added {
                let location = EntityLocation::new(
                    archetype_index,
                    chunk_set_index,
                    chunk_index,
                    ComponentIndex(i),
                );
                self.entity_locations.set(*e, location);
                self.allocation_buffer.push(*e);
            }
        }

        trace!(count = self.allocation_buffer.len(), "Inserted entities");

        &self.allocation_buffer
    }

    /// Removes the given `Entity` from the `World`.
    ///
    /// Returns `true` if the entity was deleted; else `false`.
    pub fn delete(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        if self.entity_allocator.delete_entity(entity) {
            let location = self.entity_locations.get(entity).unwrap();
            self.delete_location(location);
            trace!(world = self.id().0, ?entity, "Deleted entity");
            true
        } else {
            false
        }
    }

    /// Delete all entity data. This leaves subscriptions and the command buffer intact.
    pub fn delete_all(&mut self) {
        for archetype in self.storage_mut().archetypes_mut() {
            archetype.delete_all();
        }

        self.entity_allocator.delete_all_entities();
    }

    fn delete_location(&mut self, location: EntityLocation) {
        // find entity's chunk
        let chunk = self.storage_mut().chunk_mut(location).unwrap();

        // swap remove with last entity in chunk
        if let Some(swapped) = chunk.swap_remove(location.component(), true) {
            // record swapped entity's new location
            self.entity_locations.set(swapped, location);
        }
    }

    fn find_chunk_with_delta(
        &mut self,
        source_location: EntityLocation,
        add_components: &[(ComponentTypeId, ComponentMeta)],
        remove_components: &[ComponentTypeId],
        add_tags: &[(TagTypeId, TagMeta, NonNull<u8>)],
        remove_tags: &[TagTypeId],
    ) -> (ArchetypeIndex, SetIndex) {
        let archetype = {
            let result = {
                let source_archetype = self
                    .storage()
                    .archetype(source_location.archetype())
                    .unwrap();

                // find target chunk
                let mut component_layout = DynamicComponentLayout {
                    existing: source_archetype.description().components(),
                    add: add_components,
                    remove: remove_components,
                };

                let mut tag_layout = DynamicTagLayout {
                    storage: self.storage(),
                    archetype: source_location.archetype(),
                    set: source_location.set(),
                    existing: source_archetype.description().tags(),
                    add: add_tags,
                    remove: remove_tags,
                };

                let archetype = self.find_archetype(&mut tag_layout, &mut component_layout);
                if let Some(archetype) = archetype.as_ref() {
                    if let Some(chunk) = self.find_chunk_set(*archetype, &mut tag_layout) {
                        // fast path: chunk already exists
                        return (*archetype, chunk);
                    }

                    Ok(*archetype)
                } else {
                    let mut description = ArchetypeDescription::default();
                    component_layout.tailor_archetype(&mut description);
                    tag_layout.tailor_archetype(&mut description);

                    Err(description)
                }
            };

            match result {
                Ok(arch) => arch,
                Err(desc) => {
                    let (index, _) = unsafe { &mut *self.storage.get() }.alloc_archetype(desc);
                    index
                }
            }
        };

        // slow path: create new chunk
        let source_archetype = self
            .storage()
            .archetype(source_location.archetype())
            .unwrap();
        let mut tags = source_archetype.tags().tag_set(source_location.set());
        for type_id in remove_tags.iter() {
            tags.remove(*type_id);
        }
        for (type_id, meta, ptr) in add_tags.iter() {
            tags.push(*type_id, *meta, *ptr);
        }

        let chunk = self.create_chunk_set(archetype, &tags);

        (archetype, chunk)
    }

    fn move_entity(
        &mut self,
        entity: Entity,
        add_components: &[(ComponentTypeId, ComponentMeta)],
        remove_components: &[ComponentTypeId],
        add_tags: &[(TagTypeId, TagMeta, NonNull<u8>)],
        remove_tags: &[TagTypeId],
    ) -> &mut ComponentStorage {
        let location = self.entity_locations.get(entity).expect("entity not found");

        // find or create the target chunk
        let (target_arch_index, target_chunkset_index) = self.find_chunk_with_delta(
            location,
            add_components,
            remove_components,
            add_tags,
            remove_tags,
        );

        // Safety Note:
        // It is only safe for us to have 2 &mut references to storage here because
        // we know we are only going to be modifying two chunks that are at different
        // indexes.

        // fetch entity's chunk
        let current_chunk = unsafe { &mut *self.storage.get() }
            .chunk_mut(location)
            .unwrap();

        // fetch target chunk
        let archetype = unsafe { &mut *self.storage.get() }
            .archetype_mut(target_arch_index)
            .unwrap();
        let target_chunk_index = archetype.get_free_chunk(target_chunkset_index, 1);
        let target_chunk = unsafe {
            archetype
                .chunkset_unchecked_mut(target_chunkset_index)
                .chunk_unchecked_mut(target_chunk_index)
        };

        // move existing data over into new chunk
        if let Some(swapped) = current_chunk.move_entity(target_chunk, location.component()) {
            // update location of any entity that was moved into the previous location
            self.entity_locations.set(swapped, location);
        }

        // record the entity's new location
        self.entity_locations.set(
            entity,
            EntityLocation::new(
                target_arch_index,
                target_chunkset_index,
                target_chunk_index,
                ComponentIndex(target_chunk.len() - 1),
            ),
        );

        target_chunk
    }

    /// Adds a component to an entity, or sets its value if the component is
    /// already present.
    ///
    /// # Notes
    /// This function has the overhead of moving the entity to either an existing or new archetype,
    /// causing a memory copy of the entity to a new location. This function should not be used
    /// multiple times in successive order.
    pub fn add_component<T: Component>(
        &mut self,
        entity: Entity,
        component: T,
    ) -> Result<(), EntityMutationError> {
        if !self.is_alive(entity) {
            return Err(EntityMutationError::DoesNotExist);
        }

        if let Some(mut comp) = self.get_component_mut(entity) {
            *comp = component;
            return Ok(());
        }

        trace!(
            world = self.id().0,
            ?entity,
            component = std::any::type_name::<T>(),
            "Adding component to entity"
        );

        // move the entity into a suitable chunk
        let target_chunk = self.move_entity(
            entity,
            &[(ComponentTypeId::of::<T>(), ComponentMeta::of::<T>())],
            &[],
            &[],
            &[],
        );

        // push new component into chunk
        let mut writer = target_chunk.writer();
        let (_, components) = writer.get();
        let slice = [component];
        unsafe {
            let components = &mut *components.get();
            components
                .get_mut(ComponentTypeId::of::<T>())
                .unwrap()
                .writer()
                .push(&slice);
        }
        std::mem::forget(slice);

        Ok(())
    }

    /// Removes a component from an entity.
    ///
    /// # Notes
    /// This function has the overhead of moving the entity to either an existing or new archetype,
    /// causing a memory copy of the entity to a new location. This function should not be used
    /// multiple times in successive order.
    ///
    /// `World::remove_components` should be used for adding multiple omponents to an entity at once.
    pub fn remove_component<T: Component>(
        &mut self,
        entity: Entity,
    ) -> Result<(), EntityMutationError> {
        if !self.is_alive(entity) {
            return Err(EntityMutationError::DoesNotExist);
        }

        if self.get_component::<T>(entity).is_some() {
            trace!(
                world = self.id().0,
                ?entity,
                component = std::any::type_name::<T>(),
                "Removing component from entity"
            );

            // move the entity into a suitable chunk
            self.move_entity(entity, &[], &[ComponentTypeId::of::<T>()], &[], &[]);
        }

        Ok(())
    }

    /// Removes
    ///
    /// # Notes
    /// This function is provided for bulk deleting components from an entity. This difference between this
    /// function and `remove_component` is this allows us to remove multiple components and still only
    /// perform a single move operation of the entity.
    pub fn remove_components<T: ComponentTypeTupleSet>(
        &mut self,
        entity: Entity,
    ) -> Result<(), EntityMutationError> {
        if !self.is_alive(entity) {
            return Err(EntityMutationError::DoesNotExist);
        }

        let components = T::collect();
        for component in components.iter() {
            if !self.has_component_by_id(entity, *component) {
                return Ok(());
            }
        }

        self.move_entity(entity, &[], &components, &[], &[]);
        Ok(())
    }

    /// Adds a tag to an entity, or sets its value if the tag is
    /// already present.
    pub fn add_tag<T: Tag>(&mut self, entity: Entity, tag: T) -> Result<(), EntityMutationError> {
        if !self.is_alive(entity) {
            return Err(EntityMutationError::DoesNotExist);
        }

        if self.get_tag::<T>(entity).is_some() {
            self.remove_tag::<T>(entity)?;
        }

        trace!(
            world = self.id().0,
            ?entity,
            tag = std::any::type_name::<T>(),
            "Adding tag to entity"
        );

        // move the entity into a suitable chunk
        self.move_entity(
            entity,
            &[],
            &[],
            &[(
                TagTypeId::of::<T>(),
                TagMeta::of::<T>(),
                NonNull::new(&tag as *const _ as *mut u8).unwrap(),
            )],
            &[],
        );

        Ok(())
    }

    /// Removes a tag from an entity.
    pub fn remove_tag<T: Tag>(&mut self, entity: Entity) -> Result<(), EntityMutationError> {
        if !self.is_alive(entity) {
            return Err(EntityMutationError::DoesNotExist);
        }

        if self.get_tag::<T>(entity).is_some() {
            trace!(
                world = self.id().0,
                ?entity,
                tag = std::any::type_name::<T>(),
                "Removing tag from entity"
            );

            // move the entity into a suitable chunk
            self.move_entity(entity, &[], &[], &[], &[TagTypeId::of::<T>()]);
        }

        Ok(())
    }

    /// Borrows component data for the given entity.
    ///
    /// Returns `Some(data)` if the entity was found and contains the specified data.
    /// Otherwise `None` is returned.
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<Ref<T>> {
        if !self.is_alive(entity) {
            return None;
        }

        let location = self.entity_locations.get(entity)?;
        let chunk = self.storage().chunk(location)?;
        let (slice_borrow, slice) = unsafe {
            chunk
                .components(ComponentTypeId::of::<T>())?
                .data_slice::<T>()
                .deconstruct()
        };
        let component = slice.get(*location.component())?;

        Some(Ref::new(slice_borrow, component))
    }

    fn get_component_storage(&self, entity: Entity) -> Option<&ComponentStorage> {
        let location = self.entity_locations.get(entity)?;
        self.storage().chunk(location)
    }

    /// Checks that the provided `ComponentTypeId` is present on a given entity.
    ///
    /// Returns true if it exists, otherwise false.
    pub fn has_component_by_id(&self, entity: Entity, component: ComponentTypeId) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        if let Some(chunkset) = self.get_component_storage(entity) {
            return chunkset.components(component).is_some();
        }

        false
    }

    /// Checks that the provided `Component` is present on a given entity.
    ///
    /// Returns true if it exists, otherwise false.
    #[inline]
    pub fn has_component<T: Component>(&self, entity: Entity) -> bool {
        self.has_component_by_id(entity, ComponentTypeId::of::<T>())
    }

    /// Mutably borrows entity data for the given entity.
    ///
    /// Returns `Some(data)` if the entity was found and contains the specified data.
    /// Otherwise `None` is returned.
    ///
    /// # Safety
    ///
    /// Accessing a component which is already being concurrently accessed elsewhere is undefined behavior.
    ///
    /// # Panics
    ///
    /// This function may panic if any other code is currently borrowing `T` (such as in a query).
    pub unsafe fn get_component_mut_unchecked<T: Component>(
        &self,
        entity: Entity,
    ) -> Option<RefMut<T>> {
        if !self.is_alive(entity) {
            return None;
        }

        let location = self.entity_locations.get(entity)?;
        let chunk = self.storage().chunk(location)?;
        let (slice_borrow, slice) = chunk
            .components(ComponentTypeId::of::<T>())?
            .data_slice_mut::<T>()
            .deconstruct();
        let component = slice.get_mut(*location.component())?;

        Some(RefMut::new(slice_borrow, component))
    }

    /// Mutably borrows entity data for the given entity.
    ///
    /// Returns `Some(data)` if the entity was found and contains the specified data.
    /// Otherwise `None` is returned.
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<RefMut<T>> {
        // safe because the &mut self ensures exclusivity
        unsafe { self.get_component_mut_unchecked(entity) }
    }

    /// Gets tag data for the given entity.
    ///
    /// Returns `Some(data)` if the entity was found and contains the specified data.
    /// Otherwise `None` is returned.
    pub fn get_tag<T: Tag>(&self, entity: Entity) -> Option<&T> {
        if !self.is_alive(entity) {
            return None;
        }

        let location = self.entity_locations.get(entity)?;
        let archetype = self.storage().archetype(location.archetype())?;
        let tags = archetype.tags().get(TagTypeId::of::<T>())?;

        unsafe { tags.data_slice::<T>().get(*location.set()) }
    }

    /// Determines if the given `Entity` is alive within this `World`.
    pub fn is_alive(&self, entity: Entity) -> bool { self.entity_allocator.is_alive(entity) }

    /// Returns the entity's component types, if the entity exists.
    pub fn entity_component_types(
        &self,
        entity: Entity,
    ) -> Option<&[(ComponentTypeId, ComponentMeta)]> {
        if !self.is_alive(entity) {
            return None;
        }
        let location = self.entity_locations.get(entity);
        let archetype = location
            .map(|location| self.storage().archetype(location.archetype()))
            .unwrap_or(None);
        archetype.map(|archetype| archetype.description().components())
    }

    /// Returns the entity's tag types, if the entity exists.
    pub fn entity_tag_types(&self, entity: Entity) -> Option<&[(TagTypeId, TagMeta)]> {
        if !self.is_alive(entity) {
            return None;
        }
        let location = self.entity_locations.get(entity);
        let archetype = location
            .map(|location| self.storage().archetype(location.archetype()))
            .unwrap_or(None);
        archetype.map(|archetype| archetype.description().tags())
    }

    /// Iteratively defragments the world's internal memory.
    ///
    /// This compacts entities into fewer more continuous chunks.
    ///
    /// `budget` describes the maximum number of entities that can be moved
    /// in one call. Subsequent calls to `defrag` will resume progress from the
    /// previous call.
    pub fn defrag(&mut self, budget: Option<usize>) {
        let span = span!(
            Level::INFO,
            "Defragmenting",
            world = self.id().0,
            start_archetype = self.defrag_progress
        );
        let _guard = span.enter();

        let archetypes = unsafe { &mut *self.storage.get() }.archetypes_mut();
        let mut budget = budget.unwrap_or(std::usize::MAX);
        let start = self.defrag_progress;
        while self.defrag_progress < archetypes.len() {
            // defragment the next archetype
            let complete =
                (&mut archetypes[self.defrag_progress]).defrag(&mut budget, |e, location| {
                    self.entity_locations.set(e, location);
                });
            if complete {
                // increment the index, looping it once we get to the end
                self.defrag_progress = (self.defrag_progress + 1) % archetypes.len();
            }

            // stop once we run out of budget or reach back to where we started
            if budget == 0 || self.defrag_progress == start {
                break;
            }
        }
    }

    /// Move entities from a world to this world, copying all appropriate archetypes,
    /// tags entities and components into this world.
    pub fn move_from(&mut self, world: World) {
        let span =
            span!(Level::INFO, "Merging worlds", source = world.id().0, destination = ?self.id());
        let _guard = span.enter();

        self.entity_allocator
            .merge(Arc::try_unwrap(world.entity_allocator).unwrap());

        for archetype in unsafe { &mut *world.storage.get() }.drain(..) {
            let target_archetype = {
                // use the description as an archetype filter
                let mut desc = archetype.description().clone();
                let archetype_data = ArchetypeFilterData {
                    component_types: self.storage().component_types(),
                    tag_types: self.storage().tag_types(),
                };
                let matches = desc
                    .matches(archetype_data)
                    .matching_indices()
                    .next()
                    .map(ArchetypeIndex);
                if let Some(arch_index) = matches {
                    // similar archetype already exists, merge
                    self.storage_mut()
                        .archetype_mut(arch_index)
                        .unwrap()
                        .move_from(archetype);
                    arch_index
                } else {
                    // archetype does not already exist, append
                    self.storage_mut().push(archetype);
                    ArchetypeIndex(self.storage_mut().archetypes().len() - 1)
                }
            };

            // update entity locations
            let archetype = &unsafe { &*self.storage.get() }.archetypes()[target_archetype];
            for (entity, location) in archetype.iter_entity_locations(target_archetype) {
                self.entity_locations.set(entity, location);
            }
        }
    }

    /// This will *copy* the data from `src_world` into this world. The logic to do the copy is
    /// delegated to the `clone_impl` provided by the user. In addition to simple copying, it's also
    /// possible to transform from one type to another. This is useful for cases where you want to
    /// read from serializable data (like a physics shape definition) and construct something that
    /// isn't serializable (like a handle to a physics body)
    ///
    /// By default, all entities in the new world will be assigned a new Entity. `result_mappings`
    /// (if not None) will be populated with the old/new Entities, which allows for mapping data
    /// between the old and new world.
    ///
    /// If you want to replace existing entities (for example to hot-reload data from a file,)
    /// populate `replace_mappings`. For every entry in this map, the key must exist in the source
    /// world and the value must exist in the destination world. All entities in the destination
    /// world referenced by this map will be deleted, and the entities copied over will be assigned
    /// the same entity. If these constraints are not met, this function will panic.
    pub fn clone_from<
        's,
        CloneImplT: CloneImpl,
        CloneImplResultT: CloneImplResult,
        EntityReplacePolicyT: EntityReplacePolicy<'s>,
    >(
        &mut self,
        src_world: &World,
        clone_impl: &CloneImplT,
        clone_impl_result: &mut CloneImplResultT,
        entity_replace_policy: &'s EntityReplacePolicyT,
    ) {
        let span = span!(Level::INFO, "CloneMerging worlds", source = src_world.id().0, destination = ?self.id());
        let _guard = span.enter();

        let src_storage = unsafe { &(*src_world.storage.get()) };
        let dst_storage = unsafe { &mut (*self.storage.get()) };

        // First check that all the src entities exist in the source world. We're assuming the
        // source data will be available later to replace the data we're about to delete
        for k in entity_replace_policy.src_entities() {
            if !src_world.entity_allocator.is_alive(k) {
                panic!("clone_from assumes all replace_mapping keys exist in the source world");
            }
        }

        // Delete all the data associated with dst_entities. This leaves the
        // associated entities in a dangling state, but we'll fix this later when we copy the
        // data over
        for entity_to_replace in entity_replace_policy.dst_entities() {
            if self.entity_allocator.is_alive(entity_to_replace) {
                let location = self
                    .entity_locations
                    .get(entity_to_replace)
                    .expect("Failed to get location of live entity");
                self.delete_location(location);
            } else {
                panic!(
                    "clone_from assumes all replace_mapping values exist in the destination world"
                );
            }
        }

        // Iterate all archetypes in the src world
        for src_archetype in src_storage.archetypes() {
            let archetype_data = ArchetypeFilterData {
                component_types: self.storage().component_types(),
                tag_types: self.storage().tag_types(),
            };

            let dst_archetype_index = World::find_or_create_archetype_for_clone_move(
                clone_impl,
                src_archetype.description(),
                archetype_data,
                dst_storage,
            );

            // Do the clone_from for this archetype
            dst_storage
                .archetype_mut(dst_archetype_index)
                .unwrap()
                .clone_from(
                    &src_world,
                    src_archetype,
                    dst_archetype_index,
                    &self.entity_allocator,
                    &mut self.entity_locations,
                    clone_impl,
                    clone_impl_result,
                    entity_replace_policy,
                );
        }
    }

    /// This will *copy* the `src_entity` from `src_world` into this world. The logic to do the copy
    /// is delegated to the `clone_impl` provided by the user. In addition to simple copying, it's
    /// also possible to transform from one type to another. This is useful for cases where you want
    /// to read from serializable data (like a physics shape definition) and construct something
    /// that isn't serializable (like a handle to a physics body)
    ///
    /// By default, the entity in the new world will be assigned a new Entity. The return value
    /// indicates the Entity in the new world, which allows for mapping data the old and new world.
    ///
    /// If you want to replace an existing entity (for example to hot-reload data from a file,)
    /// populate `replace_mapping`. This entity must exist in the destination world. The entity in
    /// the destination world will be deleted, and the entity copied over will be assigned
    /// the same entity. If these constraints are not met, this function will panic.
    pub fn clone_from_single<C: CloneImpl>(
        &mut self,
        src_world: &World,
        src_entity: Entity,
        clone_impl: &C,
        replace_mapping: Option<Entity>,
    ) -> Entity {
        let span = span!(Level::INFO, "CloneMergingSingle worlds", source = src_world.id().0, destination = ?self.id());
        let _guard = span.enter();

        let src_storage = unsafe { &(*src_world.storage.get()) };
        let dst_storage = unsafe { &mut (*self.storage.get()) };

        if !src_world.entity_allocator.is_alive(src_entity) {
            panic!("src_entity not alive");
        }

        // Erase all entities that are referred to by value. The code following will update the location
        // of all these entities to point to new, valid locations
        if let Some(replace_mapping) = replace_mapping {
            if self.entity_allocator.is_alive(replace_mapping) {
                let location = self
                    .entity_locations
                    .get(replace_mapping)
                    .expect("Failed to get location of live entity");
                self.delete_location(location);
            } else {
                panic!("clone_from_single assumes entity_mapping exists in the destination world");
            }
        }

        let src_location = src_world.entity_locations.get(src_entity).unwrap();
        let src_archetype = &src_storage.archetypes()[src_location.archetype()];

        // Iterate all archetypes in the src world
        let archetype_data = ArchetypeFilterData {
            component_types: self.storage().component_types(),
            tag_types: self.storage().tag_types(),
        };

        let dst_archetype_index = World::find_or_create_archetype_for_clone_move(
            clone_impl,
            src_archetype.description(),
            archetype_data,
            dst_storage,
        );

        // Do the clone_from for this archetype
        dst_storage
            .archetype_mut(dst_archetype_index)
            .unwrap()
            .clone_from_single(
                &src_world,
                src_archetype,
                &src_location,
                dst_archetype_index,
                &self.entity_allocator,
                &mut self.entity_locations,
                clone_impl,
                replace_mapping,
            )
    }

    fn find_or_create_archetype_for_clone_move<C: CloneImpl>(
        clone_impl: &C,
        src_archetype_description: &ArchetypeDescription,
        archetype_data: ArchetypeFilterData,
        dst_storage: &mut Storage,
    ) -> ArchetypeIndex {
        // Build the archetype that we will write into. The caller of this function provides an
        // impl to do the clone, optionally transforming components from one type to another
        let mut dst_archetype = ArchetypeDescription::default();
        for (from_type_id, _from_meta) in src_archetype_description.components() {
            let (into_type_id, into_meta) = clone_impl.map_component_type(*from_type_id);
            dst_archetype.register_component_raw(into_type_id, into_meta);
        }

        // Find or create the archetype in the destination world
        let matches = dst_archetype
            .matches(archetype_data)
            .matching_indices()
            .next();

        // If it doesn't exist, allocate it
        if let Some(arch_index) = matches {
            ArchetypeIndex(arch_index)
        } else {
            dst_storage.alloc_archetype(dst_archetype).0
        }
    }

    fn find_archetype<T, C>(&self, tags: &mut T, components: &mut C) -> Option<ArchetypeIndex>
    where
        T: for<'a> Filter<ArchetypeFilterData<'a>>,
        C: for<'a> Filter<ArchetypeFilterData<'a>>,
    {
        // search for an archetype with an exact match for the desired component layout
        let archetype_data = ArchetypeFilterData {
            component_types: self.storage().component_types(),
            tag_types: self.storage().tag_types(),
        };

        // zip the two filters together - find the first index that matches both
        tags.matches(archetype_data)
            .zip(components.matches(archetype_data))
            .enumerate()
            .take(self.storage().archetypes().len())
            .filter(|(_, (a, b))| *a && *b)
            .map(|(i, _)| i)
            .next()
            .map(ArchetypeIndex)
    }

    fn create_archetype<T, C>(&mut self, tags: &T, components: &C) -> ArchetypeIndex
    where
        T: TagLayout,
        C: ComponentLayout,
    {
        let mut description = ArchetypeDescription::default();
        tags.tailor_archetype(&mut description);
        components.tailor_archetype(&mut description);

        let (index, _) = unsafe { &mut *self.storage.get() }.alloc_archetype(description);
        index
    }

    fn find_or_create_archetype<T, C>(&mut self, tags: &mut T, components: &mut C) -> ArchetypeIndex
    where
        T: TagLayout,
        C: ComponentLayout,
    {
        if let Some(i) = self.find_archetype(tags.get_filter(), components.get_filter()) {
            i
        } else {
            self.create_archetype(tags, components)
        }
    }

    fn find_chunk_set<T>(&self, archetype: ArchetypeIndex, tags: &mut T) -> Option<SetIndex>
    where
        T: for<'a> Filter<ChunksetFilterData<'a>>,
    {
        // fetch the archetype, we can already assume that the archetype index is valid
        let archetype_data = unsafe { self.storage().archetype_unchecked(archetype) };

        // find a chunk with the correct tags
        let chunk_filter_data = ChunksetFilterData {
            archetype_data: archetype_data.deref(),
        };

        if let Some(i) = tags.matches(chunk_filter_data).matching_indices().next() {
            return Some(SetIndex(i));
        }

        None
    }

    fn create_chunk_set<T>(&mut self, archetype: ArchetypeIndex, tags: &T) -> SetIndex
    where
        T: TagSet,
    {
        let archetype_data = unsafe { self.storage_mut().archetype_unchecked_mut(archetype) };
        archetype_data.alloc_chunk_set(|chunk_tags| tags.write_tags(chunk_tags))
    }

    fn find_or_create_chunk<T>(&mut self, archetype: ArchetypeIndex, tags: &mut T) -> SetIndex
    where
        T: TagSet + for<'a> Filter<ChunksetFilterData<'a>>,
    {
        if let Some(i) = self.find_chunk_set(archetype, tags) {
            i
        } else {
            self.create_chunk_set(archetype, tags)
        }
    }
}

impl Default for World {
    fn default() -> Self { Self::new() }
}

/// Describes how to handle a `clone_from`. Allows the user to transform components from one type
/// to another and provide their own implementation for cloning/transforming
pub trait CloneImpl {
    /// When a component of the provided `component_type` is encountered, we will transfer data
    /// from it into the returned component type. For a basic clone implementation, this function
    /// should return the same type as was passed into it
    fn map_component_type(
        &self,
        component_type_id: ComponentTypeId,
    ) -> (ComponentTypeId, ComponentMeta);

    /// When called, the implementation should copy the data from src_data to dst_data. The
    /// src_world and src_entities are provided so that other components on the same Entity can
    /// be looked up. The dst_resources are provided so that any required side effects to resources
    /// (like registering a physics body into a physics engine) can be implemented.
    #[allow(clippy::too_many_arguments)]
    fn clone_components(
        &self,
        src_world: &World,
        src_component_storage: &ComponentStorage,
        src_component_storage_indexes: core::ops::Range<ComponentIndex>,
        src_type: ComponentTypeId,
        src_entities: &[Entity],
        dst_entities: &[Entity],
        src_data: *const u8,
        dst_data: *mut u8,
        num_components: usize,
    );
}

/// Used along with `CloneImpl`, allows receiving results from a `clone_from` or `clone_from_single`
/// call.
pub trait CloneImplResult {
    /// For every entity that is copied, this function will be called, passing the entity in the
    /// source and destination worlds
    fn add_result(&mut self, src_entity: Entity, dst_entity: Entity);
}

/// Used along with `CloneImpl`, allows specifying that certain entities in the receiving world should
/// be replaced with entities from the source world.
///
/// A typical implementation of this trait would be to wrap a HashMap. `src_entities` would be
/// implemented by returning keys(), `dst_entities` would be implemented by returning values(), and
/// `get_dst_entity` would be implemented by returning the result of get(src_entity).
///
/// Default implementations provided in legion include:
/// * `NoneEntityReplacePolicy` - No entity replacement will occur
/// * `HashMapCloneImplResult` - Wraps the standard library's HashMap.
pub trait EntityReplacePolicy<'s> {
    /// Returns all entities in the source world that will replace data in the destination world
    ///
    /// # Safety
    ///
    /// * All entities returned via the iterator must exist in the source world
    /// * All entities that will be copied from the source world must be included in the
    ///   returned iterator.
    fn src_entities<'a>(&'s self) -> Box<dyn Iterator<Item = Entity> + 'a>
    where
        's: 'a;

    /// Returns all entities in the destination world that will be replaced
    ///
    /// # Safety
    ///
    /// * All entities returned via the iterator must exist in the destination world
    /// * All entities that will be replaced in the destination world must be included in the
    ///   returned iterator
    fn dst_entities<'a>(&'s self) -> Box<dyn Iterator<Item = Entity> + 'a>
    where
        's: 'a;

    /// Returns the entity in the destination world that will be replaced by the given entity in the
    /// source world, otherwise None if the entity in the source world should not replace anything.
    ///
    /// # Safety
    ///
    /// * All entities passed into this function that result in a non-None return value must be
    ///   included in the iterator returned by `src_entities`
    /// * All entities returned by this function must be included in the iterator returned by
    ///   `dst_entities`
    fn get_dst_entity(&self, src_entity: Entity) -> Option<Entity>;
}

/// Used to opt-out of receiving results from a `clone_from` or `clone_from_single` call
/// (See comments on `CloneImplResult`)
pub struct NoneCloneImplResult;
impl CloneImplResult for NoneCloneImplResult {
    fn add_result(&mut self, _src_entity: Entity, _dst_entity: Entity) {
        // do nothing
    }
}

/// Used to opt-out of replacing entities during a `clone_from` or `clone_from_single` call.
/// (See comments on `EntityReplacePolicy`)
pub struct NoneEntityReplacePolicy;
impl<'s> EntityReplacePolicy<'s> for NoneEntityReplacePolicy {
    fn src_entities<'a>(&self) -> Box<dyn Iterator<Item = Entity> + 'a>
    where
        's: 'a,
    {
        Box::new(std::iter::Empty::default())
    }

    fn dst_entities<'a>(&self) -> Box<dyn Iterator<Item = Entity> + 'a>
    where
        's: 'a,
    {
        Box::new(std::iter::Empty::default())
    }

    fn get_dst_entity(&self, _src_entity: Entity) -> Option<Entity> { None }
}

/// Default implementation of `CloneImplResult` that uses a hash map. Keys are entities in the
/// source world and values are entities in the destination world. (See comments on
/// `CloneImplResult`)
pub struct HashMapCloneImplResult<'m>(pub &'m mut HashMap<Entity, Entity>);

impl<'m> CloneImplResult for HashMapCloneImplResult<'m> {
    fn add_result(&mut self, src_entity: Entity, dst_entity: Entity) {
        self.0.insert(src_entity, dst_entity);
    }
}

/// Default implementation of `EntityReplacePolicy` that uses a hash map. Keys are entities in the
/// source world and values are entities in the destination world. (See comments on
/// `EntityReplacePolicy`)
pub struct HashMapEntityReplacePolicy<'m>(pub &'m HashMap<Entity, Entity>);

impl<'m, 's> EntityReplacePolicy<'s> for HashMapEntityReplacePolicy<'m> {
    fn src_entities<'a>(&'s self) -> Box<dyn Iterator<Item = Entity> + 'a>
    where
        's: 'a,
    {
        Box::new(self.0.keys().cloned())
    }

    fn dst_entities<'a>(&'s self) -> Box<dyn Iterator<Item = Entity> + 'a>
    where
        's: 'a,
    {
        Box::new(self.0.values().cloned())
    }

    fn get_dst_entity(&self, src_entity: Entity) -> Option<Entity> {
        self.0.get(&src_entity).copied()
    }
}

#[derive(Error, Debug)]
pub enum EntityMutationError {
    #[error("entity does not exist")]
    DoesNotExist,
}

/// Describes the types of a set of components attached to an entity.
pub trait ComponentLayout: Sized {
    /// A filter type which filters archetypes to an exact match with this layout.
    type Filter: for<'a> Filter<ArchetypeFilterData<'a>>;

    /// Gets the archetype filter for this layout.
    fn get_filter(&mut self) -> &mut Self::Filter;

    /// Modifies an archetype description to include the components described by this layout.
    fn tailor_archetype(&self, archetype: &mut ArchetypeDescription);
}

/// Describes the types of a set of tags attached to an entity.
pub trait TagLayout: Sized {
    /// A filter type which filters archetypes to an exact match with this layout.
    type Filter: for<'a> Filter<ArchetypeFilterData<'a>>;

    /// Gets the archetype filter for this layout.
    fn get_filter(&mut self) -> &mut Self::Filter;

    /// Modifies an archetype description to include the tags described by this layout.
    fn tailor_archetype(&self, archetype: &mut ArchetypeDescription);
}

/// A set of tag values to be attached to an entity.
pub trait TagSet {
    /// Writes the tags in this set to a new chunk.
    fn write_tags(&self, tags: &mut Tags);
}

/// A set of components to be attached to one or more entities.
pub trait ComponentSource: ComponentLayout {
    /// Determines if this component source has any more entity data to write.
    fn is_empty(&mut self) -> bool;

    /// Retrieves the nubmer of entities in this component source.
    fn len(&self) -> usize;

    /// Writes as many components as possible into a chunk.
    fn write<T: Iterator<Item = Entity>>(
        &mut self,
        entities: T,
        chunk: &mut ComponentStorage,
    ) -> usize;
}

/// An object that can be converted into a `ComponentSource`.
pub trait IntoComponentSource {
    /// The component source type that can be converted into.
    type Source: ComponentSource;

    /// Converts `self` into a component source.
    fn into(self) -> Self::Source;
}

/// A `ComponentSource` which can insert tuples of components representing each entity into a world.
pub struct ComponentTupleSet<T, I>
where
    I: Iterator<Item = T>,
{
    iter: Peekable<I>,
    filter: ComponentTupleFilter<T>,
}

impl<T, I> From<I> for ComponentTupleSet<T, I>
where
    I: Iterator<Item = T>,
    ComponentTupleSet<T, I>: ComponentSource,
{
    fn from(iter: I) -> Self {
        ComponentTupleSet {
            iter: iter.peekable(),
            filter: ComponentTupleFilter {
                _phantom: PhantomData,
            },
        }
    }
}

impl<I> IntoComponentSource for I
where
    I: IntoIterator,
    ComponentTupleSet<I::Item, I::IntoIter>: ComponentSource,
{
    type Source = ComponentTupleSet<I::Item, I::IntoIter>;

    fn into(self) -> Self::Source {
        ComponentTupleSet {
            iter: self.into_iter().peekable(),
            filter: ComponentTupleFilter {
                _phantom: PhantomData,
            },
        }
    }
}

pub struct PreallocComponentSource<I: Iterator<Item = Entity> + FusedIterator, C: ComponentSource> {
    entities: I,
    components: C,
}

impl<I: Iterator<Item = Entity> + FusedIterator, C: ComponentSource> IntoComponentSource
    for PreallocComponentSource<I, C>
{
    type Source = Self;

    fn into(self) -> Self::Source { self }
}

impl<I: Iterator<Item = Entity>, C: ComponentSource> PreallocComponentSource<Fuse<I>, C> {
    pub fn new(entities: I, components: C) -> Self {
        Self {
            entities: entities.fuse(),
            components,
        }
    }
}

impl<I: Iterator<Item = Entity> + FusedIterator, C: ComponentSource> ComponentLayout
    for PreallocComponentSource<I, C>
{
    type Filter = C::Filter;

    fn get_filter(&mut self) -> &mut Self::Filter { self.components.get_filter() }

    fn tailor_archetype(&self, archetype: &mut ArchetypeDescription) {
        self.components.tailor_archetype(archetype)
    }
}

impl<I: Iterator<Item = Entity> + FusedIterator, C: ComponentSource> ComponentSource
    for PreallocComponentSource<I, C>
{
    fn is_empty(&mut self) -> bool { self.components.is_empty() }

    fn len(&self) -> usize { self.components.len() }

    fn write<T: Iterator<Item = Entity>>(
        &mut self,
        mut entities: T,
        chunk: &mut ComponentStorage,
    ) -> usize {
        let iter = ConcatIter {
            a: &mut self.entities,
            b: &mut entities,
        };
        self.components.write(iter, chunk)
    }
}

struct ConcatIter<'a, T, A: Iterator<Item = T> + FusedIterator, B: Iterator<Item = T>> {
    a: &'a mut A,
    b: &'a mut B,
}

impl<'a, T, A: Iterator<Item = T> + FusedIterator, B: Iterator<Item = T>> Iterator
    for ConcatIter<'a, T, A, B>
{
    type Item = T;

    fn next(&mut self) -> Option<T> { self.a.next().or_else(|| self.b.next()) }
}

pub struct ComponentTupleFilter<T> {
    _phantom: PhantomData<T>,
}

pub trait ComponentTypeTupleSet {
    fn collect() -> Vec<ComponentTypeId>;
}

mod tuple_impls {
    use super::*;
    use crate::iterator::SliceVecIter;
    use crate::storage::Component;
    use crate::storage::ComponentTypeId;
    use crate::storage::Tag;
    use crate::zip::Zip;
    use std::iter::Repeat;
    use std::iter::Take;
    use std::slice::Iter;

    macro_rules! impl_data_tuple {
        ( $( $ty: ident => $id: ident ),* ) => {
            impl_data_tuple!(@TAG_SET $( $ty => $id ),*);
            impl_data_tuple!(@COMPONENT_SOURCE $( $ty => $id ),*);
        };
        ( @COMPONENT_SOURCE $( $ty: ident => $id: ident ),* ) => {
            impl<UWU, $( $ty ),*> ComponentLayout for ComponentTupleSet<($( $ty, )*), UWU>
            where
                UWU: ExactSizeIterator + Iterator<Item = ($( $ty, )*)>,
                $( $ty: Component ),*
            {
                type Filter = ComponentTupleFilter<($( $ty, )*)>;

                fn get_filter(&mut self) -> &mut Self::Filter {
                    &mut self.filter
                }

                fn tailor_archetype(&self, archetype: &mut ArchetypeDescription) {
                    #![allow(unused_variables)]
                    $(
                        archetype.register_component::<$ty>();
                    )*
                }
            }

            impl<UWU, $( $ty ),*> ComponentSource for ComponentTupleSet<($( $ty, )*), UWU>
            where
                UWU: ExactSizeIterator + Iterator<Item = ($( $ty, )*)>,
                $( $ty: Component ),*
            {
                fn is_empty(&mut self) -> bool {
                    self.iter.peek().is_none()
                }

                fn len(&self) -> usize {
                    self.iter.len()
                }
                fn write<EntityIter: Iterator<Item = Entity>>(&mut self, mut allocator: EntityIter, chunk: &mut ComponentStorage) -> usize {
                    #![allow(unused_variables)]
                    #![allow(unused_unsafe)]
                    #![allow(non_snake_case)]
                    let space = chunk.capacity() - chunk.len();
                    let mut writer = chunk.writer();
                    let (entities, components) = writer.get();
                    let mut count = 0;

                    unsafe {
                        $(
                            let mut $ty = (&mut *components.get()).get_mut(ComponentTypeId::of::<$ty>()).unwrap().writer();
                        )*

                        while let Some(($( $id, )*)) = { if count == space { None } else { self.iter.next() } } {
                            let entity = allocator.next().unwrap();
                            entities.push(entity);

                            $(
                                let slice = [$id];
                                $ty.push(&slice);
                                std::mem::forget(slice);
                            )*
                            count += 1;
                        }
                    }

                    count
                }
            }

            impl<'a, $( $ty ),*> Filter<ArchetypeFilterData<'a>> for ComponentTupleFilter<($( $ty, )*)>
            where
                $( $ty: Component ),*
            {
                type Iter = SliceVecIter<'a, ComponentTypeId>;

                fn collect(&self, source: ArchetypeFilterData<'a>) -> Self::Iter {
                    source.component_types.iter()
                }

                fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
                    let types = &[$( ComponentTypeId::of::<$ty>() ),*];
                    Some(types.len() == item.len() && types.iter().all(|t| item.contains(t)))
                }
            }

            impl<$( $ty ),*> ComponentTypeTupleSet for ($( $ty, )*)
            where
                $( $ty: Component ),*
            {
                fn collect() -> Vec<ComponentTypeId> {
                    vec![$( ComponentTypeId::of::<$ty>() ),*]
                }
            }
        };
        ( @TAG_SET $( $ty: ident => $id: ident ),* ) => {
            impl_data_tuple!(@CHUNK_FILTER $( $ty => $id ),*);

            impl<$( $ty ),*> TagSet for ($( $ty, )*)
            where
                $( $ty: Tag ),*
            {
                fn write_tags(&self, tags: &mut Tags) {
                    #![allow(unused_variables)]
                    #![allow(non_snake_case)]
                    let ($($id,)*) = self;
                    $(
                        unsafe {
                            tags.get_mut(TagTypeId::of::<$ty>())
                                .unwrap()
                                .push($id.clone())
                        };
                    )*
                }
            }

            impl <$( $ty ),*> TagLayout for ($( $ty, )*)
            where
                $( $ty: Tag ),*
            {
                type Filter = Self;

                fn get_filter(&mut self) -> &mut Self {
                    self
                }

                fn tailor_archetype(&self, archetype: &mut ArchetypeDescription) {
                    #![allow(unused_variables)]
                    $(
                        archetype.register_tag::<$ty>();
                    )*
                }
            }

            impl<'a, $( $ty ),*> Filter<ArchetypeFilterData<'a>> for ($( $ty, )*)
            where
                $( $ty: Tag ),*
            {
                type Iter = SliceVecIter<'a, TagTypeId>;

                fn collect(&self, source: ArchetypeFilterData<'a>) -> Self::Iter {
                    source.tag_types.iter()
                }

                fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
                    let types = &[$( TagTypeId::of::<$ty>() ),*];
                    Some(types.len() == item.len() && types.iter().all(|t| item.contains(t)))
                }
            }
        };
        ( @CHUNK_FILTER $( $ty: ident => $id: ident ),+ ) => {
            impl<'a, $( $ty ),*> Filter<ChunksetFilterData<'a>> for ($( $ty, )*)
            where
                $( $ty: Tag ),*
            {
                type Iter = Zip<($( Iter<'a, $ty>, )*)>;

                fn collect(&self, source: ChunksetFilterData<'a>) -> Self::Iter {
                    let iters = (
                        $(
                            unsafe {
                                source.archetype_data
                                    .tags()
                                    .get(TagTypeId::of::<$ty>())
                                    .unwrap()
                                    .data_slice::<$ty>()
                                    .iter()
                            },
                        )*

                    );

                    crate::zip::multizip(iters)
                }

                fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
                    #![allow(non_snake_case)]
                    let ($( $ty, )*) = self;
                    Some(($( &*$ty, )*).legion_eq(item))
                }
            }
        };
        ( @CHUNK_FILTER ) => {
            impl<'a> Filter<ChunksetFilterData<'a>> for () {
                type Iter = Take<Repeat<()>>;

                fn collect(&self, source: ChunksetFilterData<'a>) -> Self::Iter {
                    std::iter::repeat(()).take(source.archetype_data.len())
                }

                fn is_match(&self, _: &<Self::Iter as Iterator>::Item) -> Option<bool> {
                    Some(true)
                }
            }
        };
    }

    impl_data_tuple!();
    impl_data_tuple!(A => a);
    impl_data_tuple!(A => a, B => b);
    impl_data_tuple!(A => a, B => b, C => c);
    impl_data_tuple!(A => a, B => b, C => c, D => d);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n, O => o);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n, O => o, P => p);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n, O => o, P => p, Q => q);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n, O => o, P => p, Q => q, R => r);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n, O => o, P => p, Q => q, R => r, S => s);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n, O => o, P => p, Q => q, R => r, S => s, T => t);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n, O => o, P => p, Q => q, R => r, S => s, T => t, U => u);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n, O => o, P => p, Q => q, R => r, S => s, T => t, U => u, V => v);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n, O => o, P => p, Q => q, R => r, S => s, T => t, U => u, V => v, W => w);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n, O => o, P => p, Q => q, R => r, S => s, T => t, U => u, V => v, W => w, X => x);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n, O => o, P => p, Q => q, R => r, S => s, T => t, U => u, V => v, W => w, X => x, Y => y);
    impl_data_tuple!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l, M => m, N => n, O => o, P => p, Q => q, R => r, S => s, T => t, U => u, V => v, W => w, X => x, Y => y, Z => z);
}

struct DynamicComponentLayout<'a> {
    existing: &'a [(ComponentTypeId, ComponentMeta)],
    add: &'a [(ComponentTypeId, ComponentMeta)],
    remove: &'a [ComponentTypeId],
}

impl<'a> ComponentLayout for DynamicComponentLayout<'a> {
    type Filter = Self;

    fn get_filter(&mut self) -> &mut Self::Filter { self }

    fn tailor_archetype(&self, archetype: &mut ArchetypeDescription) {
        // copy components from existing archetype into new
        // except for those in `remove`
        let components = self
            .existing
            .iter()
            .filter(|(t, _)| !self.remove.contains(t));

        for (comp_type, meta) in components {
            archetype.register_component_raw(*comp_type, *meta);
        }

        // append components from `add`
        for (comp_type, meta) in self.add.iter() {
            archetype.register_component_raw(*comp_type, *meta);
        }
    }
}

impl<'a, 'b> Filter<ArchetypeFilterData<'b>> for DynamicComponentLayout<'a> {
    type Iter = SliceVecIter<'b, ComponentTypeId>;

    fn collect(&self, source: ArchetypeFilterData<'b>) -> Self::Iter {
        source.component_types.iter()
    }

    fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
        Some(
            item.len() == (self.existing.len() + self.add.len() - self.remove.len())
                && item.iter().all(|t| {
                    // all types are not in remove
                    !self.remove.contains(t)
                    // any are either in existing or add
                    && (self.existing.iter().any(|(x, _)| x == t)
                    || self.add.iter().any(|(x, _)| x == t))
                }),
        )
    }
}

struct DynamicTagLayout<'a> {
    storage: &'a Storage,
    archetype: ArchetypeIndex,
    set: SetIndex,
    existing: &'a [(TagTypeId, TagMeta)],
    add: &'a [(TagTypeId, TagMeta, NonNull<u8>)],
    remove: &'a [TagTypeId],
}

unsafe impl<'a> Send for DynamicTagLayout<'a> {}

unsafe impl<'a> Sync for DynamicTagLayout<'a> {}

impl<'a> TagLayout for DynamicTagLayout<'a> {
    type Filter = Self;

    fn get_filter(&mut self) -> &mut Self::Filter { self }

    fn tailor_archetype(&self, archetype: &mut ArchetypeDescription) {
        // copy tags from existing archetype into new
        // except for those in `remove`
        let tags = self
            .existing
            .iter()
            .filter(|(t, _)| !self.remove.contains(t));

        for (tag_type, meta) in tags {
            archetype.register_tag_raw(*tag_type, *meta);
        }

        // append tag from `add`
        for (tag_type, meta, _) in self.add.iter() {
            archetype.register_tag_raw(*tag_type, *meta);
        }
    }
}

impl<'a, 'b> Filter<ArchetypeFilterData<'b>> for DynamicTagLayout<'a> {
    type Iter = SliceVecIter<'b, TagTypeId>;

    fn collect(&self, source: ArchetypeFilterData<'b>) -> Self::Iter { source.tag_types.iter() }

    fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
        Some(
            item.len() == (self.existing.len() + self.add.len() - self.remove.len())
                && item.iter().all(|t| {
                    // all types are not in remove
                    !self.remove.contains(t)
                    // any are either in existing or add
                    && (self.existing.iter().any(|(x, _)| x == t)
                    || self.add.iter().any(|(x, _, _)| x == t))
                }),
        )
    }
}

impl<'a, 'b> Filter<ChunksetFilterData<'b>> for DynamicTagLayout<'a> {
    type Iter = Take<Enumerate<Repeat<&'b ArchetypeData>>>;

    fn collect(&self, source: ChunksetFilterData<'b>) -> Self::Iter {
        std::iter::repeat(source.archetype_data)
            .enumerate()
            .take(source.archetype_data.len())
    }

    fn is_match(&self, (set_index, arch): &<Self::Iter as Iterator>::Item) -> Option<bool> {
        for &(type_id, ref meta) in self.existing {
            if self.remove.contains(&type_id) {
                continue;
            }

            unsafe {
                // find the value of the tag in the source set
                let (slice_ptr, element_size, _) = self
                    .storage
                    .archetype(self.archetype)
                    .unwrap()
                    .tags()
                    .get(type_id)
                    .unwrap()
                    .data_raw();
                let current = slice_ptr.as_ptr().add(*self.set * element_size);

                // find the value of the tag in the candidate set
                let (slice_ptr, element_size, count) = arch.tags().get(type_id).unwrap().data_raw();
                debug_assert!(*set_index < count);
                let candidate = slice_ptr.as_ptr().add(set_index * element_size);

                if !meta.equals(current, candidate) {
                    return Some(false);
                }
            }
        }

        for &(type_id, meta, ptr) in self.add {
            unsafe {
                let (slice_ptr, element_size, count) = arch.tags().get(type_id).unwrap().data_raw();
                debug_assert!(*set_index < count);
                let candidate = slice_ptr.as_ptr().add(set_index * element_size);

                if !meta.equals(ptr.as_ptr(), candidate) {
                    return Some(false);
                }
            }
        }

        Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Pos(f32, f32, f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Rot(f32, f32, f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Scale(f32, f32, f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Vel(f32, f32, f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Accel(f32, f32, f32);
    #[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
    struct Model(u32);
    #[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
    struct Static;

    fn create() -> World {
        let universe = Universe::new();
        universe.create_world()
    }

    #[test]
    fn create_universe() {
        let _ = tracing_subscriber::fmt::try_init();

        Universe::new();
    }

    #[test]
    fn create_world() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        universe.create_world();
    }

    #[test]
    fn insert_many() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        struct One;
        struct Two;
        struct Three;
        struct Four;
        struct Five;
        struct Six;
        struct Seven;
        struct Eight;
        struct Nine;
        struct Ten;

        let shared = (1usize, 2f32, 3u16);
        let components = vec![
            (One, Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten),
            (One, Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten),
        ];
        world.insert(shared, components);

        assert_eq!(2, world.allocation_buffer.len());
    }

    #[test]
    fn insert_empty() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        let entity = world.insert((), vec![()])[0];
        world.add_component(entity, Pos(1., 2., 3.)).unwrap();

        let components = vec![
            (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        ];

        let entities = world.insert((), vec![(), ()]).to_vec();
        world.insert(
            (),
            PreallocComponentSource::new(
                entities.iter().copied(),
                IntoComponentSource::into(components.clone()),
            ),
        );

        for (i, e) in entities.iter().enumerate() {
            world.add_component(*e, Scale(2., 2., 2.)).unwrap();
            assert_eq!(
                components.get(i).unwrap().0,
                *world.get_component(*e).unwrap()
            );
            assert_eq!(
                components.get(i).unwrap().1,
                *world.get_component(*e).unwrap()
            );
            assert_eq!(Scale(2., 2., 2.), *world.get_component(*e).unwrap());
        }

        assert_eq!(2, world.allocation_buffer.len());
    }

    #[test]
    fn insert() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        let shared = (1usize, 2f32, 3u16);
        let components = vec![(4f32, 5u64, 6u16), (4f32, 5u64, 6u16)];
        world.insert(shared, components);

        assert_eq!(2, world.allocation_buffer.len());
    }

    #[test]
    fn get_component() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        let shared = (Static, Model(5));
        let components = vec![
            (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        ];

        world.insert(shared, components.clone());

        for (i, e) in world.allocation_buffer.iter().enumerate() {
            match world.get_component(*e) {
                Some(x) => assert_eq!(components.get(i).map(|(x, _)| x), Some(&x as &Pos)),
                None => assert_eq!(components.get(i).map(|(x, _)| x), None),
            }
            match world.get_component(*e) {
                Some(x) => assert_eq!(components.get(i).map(|(_, x)| x), Some(&x as &Rot)),
                None => assert_eq!(components.get(i).map(|(_, x)| x), None),
            }
        }
    }

    #[test]
    fn get_component_wrong_type() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        world.insert((), vec![(0f64,)]);

        let entity = *world.allocation_buffer.get(0).unwrap();

        assert!(world.get_component::<i32>(entity).is_none());
    }

    #[test]
    fn get_tag() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        let shared = (Static, Model(5));
        let components = vec![
            (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        ];

        world.insert(shared, components);

        for e in world.allocation_buffer.iter() {
            assert_eq!(&Static, world.get_tag::<Static>(*e).unwrap().deref());
            assert_eq!(&Model(5), world.get_tag::<Model>(*e).unwrap().deref());
        }
    }

    #[test]
    fn get_tag_wrong_type() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        let entity = world.insert((Static,), vec![(0f64,)])[0];

        assert!(world.get_tag::<Model>(entity).is_none());
    }

    #[test]
    fn delete() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        let shared = (Static, Model(5));
        let components = vec![
            (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        ];

        let entities = world.insert(shared, components).to_vec();

        for e in entities.iter() {
            assert!(world.get_component::<Pos>(*e).is_some());
        }

        for e in entities.iter() {
            world.delete(*e);
            assert!(world.get_component::<Pos>(*e).is_none());
        }
    }

    #[test]
    fn delete_last() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        let shared = (Static, Model(5));
        let components = vec![
            (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        ];

        let entities = world.insert(shared, components.clone()).to_vec();

        let last = *entities.last().unwrap();
        world.delete(last);

        for (i, e) in entities.iter().take(entities.len() - 1).enumerate() {
            match world.get_component(*e) {
                Some(x) => assert_eq!(components.get(i).map(|(x, _)| x), Some(&x as &Pos)),
                None => assert_eq!(components.get(i).map(|(x, _)| x), None),
            }
            match world.get_component(*e) {
                Some(x) => assert_eq!(components.get(i).map(|(_, x)| x), Some(&x as &Rot)),
                None => assert_eq!(components.get(i).map(|(_, x)| x), None),
            }
        }
    }

    #[test]
    fn delete_first() {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        let shared = (Static, Model(5));
        let components = vec![
            (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        ];

        let entities = world.insert(shared, components.clone()).to_vec();

        let first = *entities.first().unwrap();
        world.delete(first);

        for (i, e) in entities.iter().skip(1).enumerate() {
            match world.get_component(*e) {
                Some(x) => assert_eq!(components.get(i + 1).map(|(x, _)| x), Some(&x as &Pos)),
                None => assert_eq!(components.get(i + 1).map(|(x, _)| x), None),
            }
            match world.get_component(*e) {
                Some(x) => assert_eq!(components.get(i + 1).map(|(_, x)| x), Some(&x as &Rot)),
                None => assert_eq!(components.get(i + 1).map(|(_, x)| x), None),
            }
        }
    }

    #[test]
    fn add_component() -> Result<(), EntityMutationError> {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        let components = vec![
            (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        ];

        let entities = world.insert((Static,), components.clone()).to_vec();

        for (i, e) in entities.iter().enumerate() {
            world.add_component(*e, Scale(2., 2., 2.))?;
            assert_eq!(
                components.get(i).unwrap().0,
                *world.get_component(*e).unwrap()
            );
            assert_eq!(
                components.get(i).unwrap().1,
                *world.get_component(*e).unwrap()
            );
            assert_eq!(Scale(2., 2., 2.), *world.get_component(*e).unwrap());
        }

        Ok(())
    }

    #[test]
    fn remove_component() -> Result<(), EntityMutationError> {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        let components = vec![
            (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        ];

        let entities = world.insert((Static,), components.clone()).to_vec();

        for (i, e) in entities.iter().enumerate() {
            world.remove_component::<Rot>(*e)?;
            assert_eq!(
                components.get(i).unwrap().0,
                *world.get_component(*e).unwrap()
            );
            assert!(world.get_component::<Rot>(*e).is_none());
        }

        Ok(())
    }

    #[test]
    fn add_tag() -> Result<(), EntityMutationError> {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        let components = vec![
            (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        ];

        let entities = world.insert((Static,), components.clone()).to_vec();

        for (i, e) in entities.iter().enumerate() {
            world.add_tag(*e, Model(2))?;
            assert_eq!(
                components.get(i).unwrap().0,
                *world.get_component(*e).unwrap()
            );
            assert_eq!(
                components.get(i).unwrap().1,
                *world.get_component(*e).unwrap()
            );
            assert_eq!(Static, *world.get_tag(*e).unwrap());
            assert_eq!(Model(2), *world.get_tag(*e).unwrap());
        }

        Ok(())
    }

    #[test]
    fn remove_tag() -> Result<(), EntityMutationError> {
        let _ = tracing_subscriber::fmt::try_init();

        let mut world = create();

        let components = vec![
            (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
        ];

        let entities = world.insert((Static,), components.clone()).to_vec();

        for (i, e) in entities.iter().enumerate() {
            world.remove_tag::<Static>(*e)?;
            assert_eq!(
                components.get(i).unwrap().0,
                *world.get_component(*e).unwrap()
            );
            assert_eq!(
                components.get(i).unwrap().1,
                *world.get_component(*e).unwrap()
            );
            assert!(world.get_tag::<Static>(*e).is_none());
        }
        Ok(())
    }

    #[test]
    fn add_component2() {
        let _ = tracing_subscriber::fmt::try_init();
        struct Transform {
            translation: Vec<f32>,
        }
        let mut world = create();
        let entity = world.insert((5u32,), vec![(3u32,)])[0];
        world
            .add_component::<Transform>(
                entity,
                Transform {
                    translation: vec![0., 1., 2.],
                },
            )
            .unwrap();
    }

    #[test]
    fn move_from() {
        let universe = Universe::new();
        let mut a = universe.create_world();
        let mut b = universe.create_world();

        let entity_a = a.insert(
            (),
            vec![
                (Pos(1., 2., 3.), Rot(0.1, 0.2, 0.3)),
                (Pos(4., 5., 6.), Rot(0.4, 0.5, 0.6)),
            ],
        )[0];

        let entity_b = b.insert(
            (),
            vec![
                (Pos(7., 8., 9.), Rot(0.7, 0.8, 0.9)),
                (Pos(10., 11., 12.), Rot(0.10, 0.11, 0.12)),
            ],
        )[0];

        b.move_from(a);

        assert_eq!(*b.get_component::<Pos>(entity_b).unwrap(), Pos(7., 8., 9.));
        assert_eq!(*b.get_component::<Pos>(entity_a).unwrap(), Pos(1., 2., 3.));
    }
}
