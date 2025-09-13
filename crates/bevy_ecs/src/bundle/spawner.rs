use core::ptr::NonNull;

use bevy_ptr::{ConstNonNull, MovingPtr};

use crate::{
    archetype::{Archetype, ArchetypeCreated, ArchetypeId, SpawnBundleStatus},
    bundle::{Bundle, BundleId, BundleInfo, DynamicBundle, InsertMode},
    change_detection::MaybeLocation,
    component::Tick,
    entity::{Entities, Entity, EntityLocation},
    event::EntityComponentsTrigger,
    lifecycle::{Add, Insert, ADD, INSERT},
    relationship::RelationshipHookMode,
    storage::Table,
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

// SAFETY: We have exclusive world access so our pointers can't be invalidated externally
pub(crate) struct BundleSpawner<'w> {
    world: UnsafeWorldCell<'w>,
    bundle_info: ConstNonNull<BundleInfo>,
    table: NonNull<Table>,
    archetype: NonNull<Archetype>,
    change_tick: Tick,
}

impl<'w> BundleSpawner<'w> {
    #[inline]
    pub fn new<T: Bundle>(world: &'w mut World, change_tick: Tick) -> Self {
        let bundle_id = world.register_bundle_info::<T>();

        // SAFETY: we initialized this bundle_id in `init_info`
        unsafe { Self::new_with_id(world, bundle_id, change_tick) }
    }

    /// Creates a new [`BundleSpawner`].
    ///
    /// # Safety
    /// Caller must ensure that `bundle_id` exists in `world.bundles`
    #[inline]
    pub(crate) unsafe fn new_with_id(
        world: &'w mut World,
        bundle_id: BundleId,
        change_tick: Tick,
    ) -> Self {
        let bundle_info = world.bundles.get_unchecked(bundle_id);
        let (new_archetype_id, is_new_created) = bundle_info.insert_bundle_into_archetype(
            &mut world.archetypes,
            &mut world.storages,
            &world.components,
            &world.observers,
            ArchetypeId::EMPTY,
        );

        let archetype = &mut world.archetypes[new_archetype_id];
        let table = &mut world.storages.tables[archetype.table_id()];
        let spawner = Self {
            bundle_info: bundle_info.into(),
            table: table.into(),
            archetype: archetype.into(),
            change_tick,
            world: world.as_unsafe_world_cell(),
        };
        if is_new_created {
            spawner
                .world
                .into_deferred()
                .trigger(ArchetypeCreated(new_archetype_id));
        }
        spawner
    }

    #[inline]
    pub fn reserve_storage(&mut self, additional: usize) {
        // SAFETY: There are no outstanding world references
        let (archetype, table) = unsafe { (self.archetype.as_mut(), self.table.as_mut()) };
        archetype.reserve(additional);
        table.reserve(additional);
    }

    /// # Safety
    /// - `entity` must be allocated (but non-existent),
    /// - `T` must match this [`BundleSpawner`]'s type
    /// - If `T::Effect: !NoBundleEffect.`, then [`apply_effect`] must  be called exactly once on `bundle`
    ///   after this function returns before returning to safe code.
    /// - The value pointed to by `bundle` must not be accessed for anything other than [`apply_effect`]
    ///   or dropped.
    ///
    /// [`apply_effect`]: crate::bundle::DynamicBundle::apply_effect
    #[inline]
    #[track_caller]
    pub unsafe fn spawn_non_existent<T: DynamicBundle>(
        &mut self,
        entity: Entity,
        bundle: MovingPtr<'_, T>,
        caller: MaybeLocation,
    ) -> EntityLocation {
        // SAFETY: We do not make any structural changes to the archetype graph through self.world so these pointers always remain valid
        let bundle_info = self.bundle_info.as_ref();
        let location = {
            let table = self.table.as_mut();
            let archetype = self.archetype.as_mut();

            // SAFETY: Mutable references do not alias and will be dropped after this block
            let (sparse_sets, entities) = {
                let world = self.world.world_mut();
                (&mut world.storages.sparse_sets, &mut world.entities)
            };
            let table_row = table.allocate(entity);
            let location = archetype.allocate(entity, table_row);
            bundle_info.write_components(
                table,
                sparse_sets,
                &SpawnBundleStatus,
                bundle_info.required_component_constructors.iter(),
                entity,
                table_row,
                self.change_tick,
                bundle,
                InsertMode::Replace,
                caller,
            );
            entities.set(entity.index(), Some(location));
            entities.mark_spawn_despawn(entity.index(), caller, self.change_tick);
            location
        };

        // SAFETY: We have no outstanding mutable references to world as they were dropped
        let mut deferred_world = unsafe { self.world.into_deferred() };
        // SAFETY: `DeferredWorld` cannot provide mutable access to `Archetypes`.
        let archetype = self.archetype.as_ref();
        // SAFETY: All components in the bundle are guaranteed to exist in the World
        // as they must be initialized before creating the BundleInfo.
        unsafe {
            deferred_world.trigger_on_add(
                archetype,
                entity,
                bundle_info.iter_contributed_components(),
                caller,
            );
            if archetype.has_add_observer() {
                // SAFETY: the ADD event_key corresponds to the Add event's type
                deferred_world.trigger_raw(
                    ADD,
                    &mut Add { entity },
                    &mut EntityComponentsTrigger {
                        components: bundle_info.contributed_components(),
                    },
                    caller,
                );
            }
            deferred_world.trigger_on_insert(
                archetype,
                entity,
                bundle_info.iter_contributed_components(),
                caller,
                RelationshipHookMode::Run,
            );
            if archetype.has_insert_observer() {
                // SAFETY: the INSERT event_key corresponds to the Insert event's type
                deferred_world.trigger_raw(
                    INSERT,
                    &mut Insert { entity },
                    &mut EntityComponentsTrigger {
                        components: bundle_info.contributed_components(),
                    },
                    caller,
                );
            }
        };

        location
    }

    /// # Safety
    /// - `T` must match this [`BundleSpawner`]'s type
    /// - If `T::Effect: !NoBundleEffect.`, then [`apply_effect`] must  be called exactly once on `bundle`
    ///   after this function returns before returning to safe code.
    /// - The value pointed to by `bundle` must not be accessed for anything other than [`apply_effect`]
    ///   or dropped.
    ///
    /// [`apply_effect`]: crate::bundle::DynamicBundle::apply_effect
    #[inline]
    pub unsafe fn spawn<T: Bundle>(
        &mut self,
        bundle: MovingPtr<'_, T>,
        caller: MaybeLocation,
    ) -> Entity {
        let entity = self.entities().alloc();
        // SAFETY:
        // - `entity` is allocated above
        // - The caller ensures that `T` matches this `BundleSpawner`'s type.
        // - The caller ensures that if `T::Effect: !NoBundleEffect.`, then [`apply_effect`] must  be called exactly once on `bundle`
        //   after this function returns before returning to safe code.
        // - The caller ensures that the value pointed to by `bundle` must not be accessed for anything other than [`apply_effect`]
        //   or dropped.
        unsafe { self.spawn_non_existent::<T>(entity, bundle, caller) };
        entity
    }

    #[inline]
    pub(crate) fn entities(&mut self) -> &mut Entities {
        // SAFETY: No outstanding references to self.world, changes to entities cannot invalidate our internal pointers
        unsafe { &mut self.world.world_mut().entities }
    }

    /// # Safety
    /// - `Self` must be dropped after running this function as it may invalidate internal pointers.
    #[inline]
    pub(crate) unsafe fn flush_commands(&mut self) {
        // SAFETY: pointers on self can be invalidated,
        self.world.world_mut().flush();
    }
}
