mod command;
mod query;
mod resource;

use bevy_utils::HashSet;

use crate::{
    component::{Component, ComponentId},
    prelude::Entity,
    query::FilteredAccess,
    system::{Command, Despawn, Remove},
    world::{append_list::AppendList, world_cell::command::CellInsert, World},
};
use std::{
    any::TypeId,
    cell::RefCell,
    collections::{hash_map::Entry::Occupied, HashMap},
    rc::Rc,
};

use self::command::CellEntityCommands;
pub use self::query::{CellQuery, QueryToken};
use self::query::{FetchRefs, QueryCacheEntry};
use self::resource::ArchetypeComponentAccess;

/// Exposes safe mutable access to multiple resources at a time in a World. Attempting to access
/// World in a way that violates Rust's mutability rules will panic thanks to runtime checks.
pub struct WorldCell<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) state: WorldCellState,
}

pub(crate) struct WorldCellState {
    resource_access: RefCell<ArchetypeComponentAccess>,
    query_cache: HashMap<TypeId, Rc<QueryCacheEntry>, fxhash::FxBuildHasher>,
    /// Queries that were activated at least once in the current WorldCell session.
    query_cache_working_set: RefCell<Vec<Rc<QueryCacheEntry>>>,
    command_queue: CellCommandQueue,
    current_query_refs: FetchRefs,
}

impl WorldCellState {
    // cannot be const because of hashmap, but should still be optimized out
    #[inline]
    pub fn new() -> Self {
        Self {
            resource_access: RefCell::new(ArchetypeComponentAccess::new()),
            // component_access: RefCell::new(ComponentAccess::new()),
            query_cache: HashMap::default(),
            query_cache_working_set: Default::default(),
            command_queue: Default::default(),
            current_query_refs: Default::default(),
        }
    }

    fn get_live_query_conflicts_filtered(
        &self,
        filtered_access: &FilteredAccess<ComponentId>,
    ) -> Vec<ComponentId> {
        for query in self.query_cache_working_set.borrow().iter() {
            if let Some(current_filtered_access) = query.alive_filtered_access() {
                if !current_filtered_access.is_compatible(filtered_access) {
                    return current_filtered_access
                        .access()
                        .get_conflicts(filtered_access.access());
                }
            }
        }
        Vec::new()
    }
}

// how to merge real result with overlay?
// how to handle inserts that results in query visiting new element?
// first: prepare set of types that influence query
//  - how to handle Without<T>? Only deletions (how?) and inserts (filter out) matter
//  - how to handle With<T>? only deletions (filter out) and inserts (how?) matter
//
// create a temp world that only contains the affected entities as clones?
// create a structure that describes the "diff" internal structure as a pass-through API

#[derive(Default)]
pub struct WorldOverlay {
    touched_entities: HashSet<Entity>,
    inserted: HashMap<Entity, Vec<(ComponentId, usize)>>,
    removed: HashMap<Entity, Vec<ComponentId>>,
    despawned_entities: HashSet<Entity>,
}

impl WorldOverlay {
    fn potential_new_entities(&mut self, access: &FilteredAccess<ComponentId>) -> HashSet<Entity> {
        let mut potential_new_entities = HashSet::default();
        for (entity, components) in &self.inserted {
            for (id, _) in components {
                if access.with().contains(id.index())
                    || access.access().has_read(*id)
                    || access.access().has_write(*id)
                {
                    potential_new_entities.insert(*entity);
                    break;
                }
            }
        }
        for (entity, components) in &self.removed {
            for id in components {
                if access.without().contains(id.index()) {
                    potential_new_entities.insert(*entity);
                    break;
                }
            }
        }
        potential_new_entities
    }
}

pub trait CellCommand: Command {
    fn apply_overlay(
        &self,
        self_index: usize,
        overlay: &mut WorldOverlay,
        world: &World,
        access: &FilteredAccess<ComponentId>,
    );
}

impl<T: Component> CellCommand for CellInsert<T> {
    fn apply_overlay(
        &self,
        self_index: usize,
        overlay: &mut WorldOverlay,
        world: &World,
        access: &FilteredAccess<ComponentId>,
    ) {
        if let Some(id) = world.components().get_id(TypeId::of::<T>()) {
            if access.with().contains(id.index())
                || access.without().contains(id.index())
                || access.access().has_read(id)
                || access.access().has_write(id)
            {
                overlay.touched_entities.insert(self.entity);
                if let Occupied(mut entry) = overlay.removed.entry(self.entity) {
                    let v = entry.get_mut();
                    v.retain(|c_id| *c_id != id);
                    if v.is_empty() {
                        entry.remove();
                    }
                }
                overlay
                    .inserted
                    .entry(self.entity)
                    .and_modify(|v| match v.iter_mut().find(|(c_id, _)| *c_id == id) {
                        Some((_, overlay)) => *overlay = self_index,
                        None => v.push((id, self_index)),
                    })
                    .or_insert_with(|| vec![(id, self_index)]);
            }
        }
    }
}

impl<T: Component> CellCommand for Remove<T> {
    fn apply_overlay(
        &self,
        _self_index: usize,
        overlay: &mut WorldOverlay,
        world: &World,
        access: &FilteredAccess<ComponentId>,
    ) {
        if let Some(id) = world.components().get_id(TypeId::of::<T>()) {
            if access.with().contains(id.index())
                || access.without().contains(id.index())
                || access.access().has_read(id)
                || access.access().has_write(id)
            {
                overlay.touched_entities.insert(self.entity);
                if let Occupied(mut entry) = overlay.inserted.entry(self.entity) {
                    let v = entry.get_mut();
                    v.retain(|(c_id, _)| *c_id != id);
                    if v.is_empty() {
                        entry.remove();
                    }
                }
                overlay
                    .removed
                    .entry(self.entity)
                    .and_modify(|v| {
                        if !v.contains(&id) {
                            v.push(id);
                        }
                    })
                    .or_insert_with(|| vec![id]);
            }
        }
    }
}

impl CellCommand for Despawn {
    fn apply_overlay(
        &self,
        _self_index: usize,
        overlay: &mut WorldOverlay,
        _world: &World,
        _access: &FilteredAccess<ComponentId>,
    ) {
        overlay.touched_entities.insert(self.entity);
        overlay.despawned_entities.insert(self.entity);
        overlay.inserted.remove(&self.entity);
        overlay.removed.remove(&self.entity);
    }
}

struct CellCommandMeta {
    ptr: *mut u8,
    write: unsafe fn(value: *mut u8, world: &mut World),
    apply_overlay: unsafe fn(
        value: *const u8,
        self_index: usize,
        overlay: &mut WorldOverlay,
        world: &World,
        access: &FilteredAccess<ComponentId>,
    ),
}

/// A queue of [`CellCommand`]s
//
// NOTE: See [`CommandQueue`] as an analog for normal commands.
#[derive(Default)]
pub struct CellCommandQueue {
    bump: bumpalo::Bump,
    metas: AppendList<CellCommandMeta>,
}

// SAFE: All commands [`Command`] implement [`Send`]
unsafe impl Send for CellCommandQueue {}

// SAFE: `&CommandQueue` never gives access to the inner commands.
unsafe impl Sync for CellCommandQueue {}

impl CellCommandQueue {
    /// Push a [`Command`] onto the queue.
    #[inline]
    pub fn push<C>(&self, command: C)
    where
        C: CellCommand,
    {
        /// SAFE: This function is only every called when the `command` bytes is the associated
        /// [`Commands`] `T` type. Also this only reads the data via `read_unaligned` so unaligned
        /// accesses are safe.
        unsafe fn write_command<T: CellCommand>(command: *mut u8, world: &mut World) {
            let command = command.cast::<T>().read_unaligned();
            command.write(world);
        }

        /// SAFE: This function is only every called when the `command` bytes is the associated
        /// [`Commands`] `T` type. Also this only reads the data via `read_unaligned` so unaligned
        /// accesses are safe.
        unsafe fn apply_overlay_command<T: CellCommand>(
            command: *const u8,
            self_index: usize,
            overlay: &mut WorldOverlay,
            world: &World,
            access: &FilteredAccess<ComponentId>,
        ) {
            let command = command.cast::<T>().as_ref().unwrap();
            command.apply_overlay(self_index, overlay, world, access);
        }

        let command = self.bump.alloc(command);
        self.metas.push(CellCommandMeta {
            ptr: command as *mut C as *mut u8,
            write: write_command::<C>,
            apply_overlay: apply_overlay_command::<C>,
        });
    }

    /// SAFETY: must know that nth command is of type C
    pub(crate) unsafe fn get_nth<C: Command>(&self, index: usize) -> &C {
        let meta = &self.metas[index];
        meta.ptr.cast::<C>().as_ref().unwrap()
    }

    /// Execute the queued [`Command`]s in the world.
    /// This clears the queue.
    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        // flush the previously queued entities
        world.flush();

        // SAFE: In the iteration below, `meta.func` will safely consume and drop each pushed command.
        // This operation is so that we can reuse the bytes `Vec<u8>`'s internal storage and prevent
        // unnecessary allocations.

        for meta in self.metas.drain() {
            // SAFE: The implementation of `write_command` is safe for the according Command type.
            // The bytes are safely cast to their original type, safely read, and then dropped.
            unsafe {
                (meta.write)(meta.ptr, world);
            }
        }
        self.bump.reset();
    }

    /// Execute the queued [`Command`]s in the world.
    /// This clears the queue.
    #[inline]
    pub fn apply_overlay(
        &self,
        overlay: &mut WorldOverlay,
        world: &World,
        access: &FilteredAccess<ComponentId>,
    ) {
        for (index, meta) in self.metas.iter().enumerate() {
            // SAFE: The implementation of `apply_overlay_command` is safe for the according Command type.
            // The bytes are safely cast to their original type and safely dereferenced.
            unsafe {
                (meta.apply_overlay)(meta.ptr, index, overlay, world, access);
            }
        }
    }
}

impl<'w> Drop for WorldCell<'w> {
    fn drop(&mut self) {
        self.maintain();

        // give world WorldCellState back to reuse allocations
        let _ = std::mem::swap(&mut self.world.world_cell_state, &mut self.state);
    }
}

impl<'w> WorldCell<'w> {
    pub(crate) fn new(world: &'w mut World) -> Self {
        // this is cheap because WorldCellState::new() is const / allocation free
        let state = std::mem::replace(&mut world.world_cell_state, WorldCellState::new());
        // world's WorldCellState is recycled to cut down on allocations
        Self { world, state }
    }

    pub fn spawn(&self) -> CellEntityCommands<'_> {
        self.entity(self.world.entities.reserve_entity())
    }
}
