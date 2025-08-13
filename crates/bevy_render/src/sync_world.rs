use bevy_app::Plugin;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::entity::EntityHash;
use bevy_ecs::lifecycle::{Add, Remove};
use bevy_ecs::{
    component::Component,
    entity::{ContainsEntity, Entity, EntityEquivalent},
    observer::On,
    query::With,
    reflect::ReflectComponent,
    resource::Resource,
    system::{Local, Query, ResMut, SystemState},
    world::{Mut, World},
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// A plugin that synchronizes entities with [`SyncToRenderWorld`] between the main world and the render world.
///
/// All entities with the [`SyncToRenderWorld`] component are kept in sync. It
/// is automatically added as a required component by [`ExtractComponentPlugin`]
/// and [`SyncComponentPlugin`], so it doesn't need to be added manually when
/// spawning or as a required component when either of these plugins are used.
///
/// # Implementation
///
/// Bevy's renderer is architected independently from the main app.
/// It operates in its own separate ECS [`World`], so the renderer logic can run in parallel with the main world logic.
/// This is called "Pipelined Rendering", see [`PipelinedRenderingPlugin`] for more information.
///
/// [`SyncWorldPlugin`] is the first thing that runs every frame and it maintains an entity-to-entity mapping
/// between the main world and the render world.
/// It does so by spawning and despawning entities in the render world, to match spawned and despawned entities in the main world.
/// The link between synced entities is maintained by the [`RenderEntity`] and [`MainEntity`] components.
///
/// The [`RenderEntity`] contains the corresponding render world entity of a main world entity, while [`MainEntity`] contains
/// the corresponding main world entity of a render world entity.
/// For convenience, [`QueryData`](bevy_ecs::query::QueryData) implementations are provided for both components:
/// adding [`MainEntity`] to a query (without a `&`) will return the corresponding main world [`Entity`],
/// and adding [`RenderEntity`] will return the corresponding render world [`Entity`].
/// If you have access to the component itself, the underlying entities can be accessed by calling `.id()`.
///
/// Synchronization is necessary preparation for extraction ([`ExtractSchedule`](crate::ExtractSchedule)), which copies over component data from the main
/// to the render world for these entities.
///
/// ```text
/// |--------------------------------------------------------------------|
/// |      |         |          Main world update                        |
/// | sync | extract |---------------------------------------------------|
/// |      |         |         Render world update                       |
/// |--------------------------------------------------------------------|
/// ```
///
/// An example for synchronized main entities 1v1 and 18v1
///
/// ```text
/// |---------------------------Main World------------------------------|
/// |  Entity  |                    Component                           |
/// |-------------------------------------------------------------------|
/// | ID: 1v1  | PointLight | RenderEntity(ID: 3V1) | SyncToRenderWorld |
/// | ID: 18v1 | PointLight | RenderEntity(ID: 5V1) | SyncToRenderWorld |
/// |-------------------------------------------------------------------|
///
/// |----------Render World-----------|
/// |  Entity  |       Component      |
/// |---------------------------------|
/// | ID: 3v1  | MainEntity(ID: 1V1)  |
/// | ID: 5v1  | MainEntity(ID: 18V1) |
/// |---------------------------------|
///
/// ```
///
/// Note that this effectively establishes a link between the main world entity and the render world entity.
/// Not every entity needs to be synchronized, however; only entities with the [`SyncToRenderWorld`] component are synced.
/// Adding [`SyncToRenderWorld`] to a main world component will establish such a link.
/// Once a synchronized main entity is despawned, its corresponding render entity will be automatically
/// despawned in the next `sync`.
///
/// The sync step does not copy any of component data between worlds, since its often not necessary to transfer over all
/// the components of a main world entity.
/// The render world probably cares about a `Position` component, but not a `Velocity` component.
/// The extraction happens in its own step, independently from, and after synchronization.
///
/// Moreover, [`SyncWorldPlugin`] only synchronizes *entities*. [`RenderAsset`](crate::render_asset::RenderAsset)s like meshes and textures are handled
/// differently.
///
/// [`PipelinedRenderingPlugin`]: crate::pipelined_rendering::PipelinedRenderingPlugin
/// [`ExtractComponentPlugin`]: crate::extract_component::ExtractComponentPlugin
/// [`SyncComponentPlugin`]: crate::sync_component::SyncComponentPlugin
#[derive(Default)]
pub struct SyncWorldPlugin;

impl Plugin for SyncWorldPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<PendingSyncEntity>();
        app.add_observer(
            |trigger: On<Add, SyncToRenderWorld>, mut pending: ResMut<PendingSyncEntity>| {
                pending.push(EntityRecord::Added(trigger.target()));
            },
        );
        app.add_observer(
            |trigger: On<Remove, SyncToRenderWorld>,
             mut pending: ResMut<PendingSyncEntity>,
             query: Query<&RenderEntity>| {
                if let Ok(e) = query.get(trigger.target()) {
                    pending.push(EntityRecord::Removed(*e));
                };
            },
        );
    }
}
/// Marker component that indicates that its entity needs to be synchronized to the render world.
///
/// This component is automatically added as a required component by [`ExtractComponentPlugin`] and [`SyncComponentPlugin`].
/// For more information see [`SyncWorldPlugin`].
///
/// NOTE: This component should persist throughout the entity's entire lifecycle.
/// If this component is removed from its entity, the entity will be despawned.
///
/// [`ExtractComponentPlugin`]: crate::extract_component::ExtractComponentPlugin
/// [`SyncComponentPlugin`]: crate::sync_component::SyncComponentPlugin
#[derive(Component, Copy, Clone, Debug, Default, Reflect)]
#[reflect[Component, Default, Clone]]
#[component(storage = "SparseSet")]
pub struct SyncToRenderWorld;

/// Component added on the main world entities that are synced to the Render World in order to keep track of the corresponding render world entity.
///
/// Can also be used as a newtype wrapper for render world entities.
#[derive(Component, Deref, Copy, Clone, Debug, Eq, Hash, PartialEq, Reflect)]
#[component(clone_behavior = Ignore)]
#[reflect(Component, Clone)]
pub struct RenderEntity(Entity);
impl RenderEntity {
    #[inline]
    pub fn id(&self) -> Entity {
        self.0
    }
}

impl From<Entity> for RenderEntity {
    fn from(entity: Entity) -> Self {
        RenderEntity(entity)
    }
}

impl ContainsEntity for RenderEntity {
    fn entity(&self) -> Entity {
        self.id()
    }
}

// SAFETY: RenderEntity is a newtype around Entity that derives its comparison traits.
unsafe impl EntityEquivalent for RenderEntity {}

/// Component added on the render world entities to keep track of the corresponding main world entity.
///
/// Can also be used as a newtype wrapper for main world entities.
#[derive(Component, Deref, Copy, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Reflect)]
#[reflect(Component, Clone)]
pub struct MainEntity(Entity);
impl MainEntity {
    #[inline]
    pub fn id(&self) -> Entity {
        self.0
    }
}

impl From<Entity> for MainEntity {
    fn from(entity: Entity) -> Self {
        MainEntity(entity)
    }
}

impl ContainsEntity for MainEntity {
    fn entity(&self) -> Entity {
        self.id()
    }
}

// SAFETY: RenderEntity is a newtype around Entity that derives its comparison traits.
unsafe impl EntityEquivalent for MainEntity {}

/// A [`HashMap`] pre-configured to use [`EntityHash`] hashing with a [`MainEntity`].
pub type MainEntityHashMap<V> = HashMap<MainEntity, V, EntityHash>;

/// A [`HashSet`] pre-configured to use [`EntityHash`] hashing with a [`MainEntity`]..
pub type MainEntityHashSet = HashSet<MainEntity, EntityHash>;

/// Marker component that indicates that its entity needs to be despawned at the end of the frame.
#[derive(Component, Copy, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct TemporaryRenderEntity;

/// A record enum to what entities with [`SyncToRenderWorld`] have been added or removed.
#[derive(Debug)]
pub(crate) enum EntityRecord {
    /// When an entity is spawned on the main world, notify the render world so that it can spawn a corresponding
    /// entity. This contains the main world entity.
    Added(Entity),
    /// When an entity is despawned on the main world, notify the render world so that the corresponding entity can be
    /// despawned. This contains the render world entity.
    Removed(RenderEntity),
    /// When a component is removed from an entity, notify the render world so that the corresponding component can be
    /// removed. This contains the main world entity.
    ComponentRemoved(Entity),
}

// Entity Record in MainWorld pending to Sync
#[derive(Resource, Default, Deref, DerefMut)]
pub(crate) struct PendingSyncEntity {
    records: Vec<EntityRecord>,
}

pub(crate) fn entity_sync_system(main_world: &mut World, render_world: &mut World) {
    main_world.resource_scope(|world, mut pending: Mut<PendingSyncEntity>| {
        // TODO : batching record
        for record in pending.drain(..) {
            match record {
                EntityRecord::Added(e) => {
                    if let Ok(mut main_entity) = world.get_entity_mut(e) {
                        match main_entity.entry::<RenderEntity>() {
                            bevy_ecs::world::ComponentEntry::Occupied(_) => {
                                panic!("Attempting to synchronize an entity that has already been synchronized!");
                            }
                            bevy_ecs::world::ComponentEntry::Vacant(entry) => {
                                let id = render_world.spawn(MainEntity(e)).id();

                                entry.insert(RenderEntity(id));
                            }
                        };
                    }
                }
                EntityRecord::Removed(render_entity) => {
                    if let Ok(ec) = render_world.get_entity_mut(render_entity.id()) {
                        ec.despawn();
                    };
                }
                EntityRecord::ComponentRemoved(main_entity) => {
                    let Some(mut render_entity) = world.get_mut::<RenderEntity>(main_entity) else {
                        continue;
                    };
                    if let Ok(render_world_entity) = render_world.get_entity_mut(render_entity.id()) {
                        // In order to handle components that extract to derived components, we clear the entity
                        // and let the extraction system re-add the components.
                        render_world_entity.despawn();

                        let id = render_world.spawn(MainEntity(main_entity)).id();
                        render_entity.0 = id;
                    }
                },
            }
        }
    });
}

pub(crate) fn despawn_temporary_render_entities(
    world: &mut World,
    state: &mut SystemState<Query<Entity, With<TemporaryRenderEntity>>>,
    mut local: Local<Vec<Entity>>,
) {
    let query = state.get(world);

    local.extend(query.iter());

    // Ensure next frame allocation keeps order
    local.sort_unstable_by_key(|e| e.index());
    for e in local.drain(..).rev() {
        world.despawn(e);
    }
}

/// This module exists to keep the complex unsafe code out of the main module.
///
/// The implementations for both [`MainEntity`] and [`RenderEntity`] should stay in sync,
/// and are based off of the `&T` implementation in `bevy_ecs`.
mod render_entities_world_query_impls {
    use super::{MainEntity, RenderEntity};

    use bevy_ecs::{
        archetype::Archetype,
        component::{ComponentId, Components, Tick},
        entity::Entity,
        query::{FilteredAccess, QueryData, ReadOnlyQueryData, ReleaseStateQueryData, WorldQuery},
        storage::{Table, TableRow},
        world::{unsafe_world_cell::UnsafeWorldCell, World},
    };

    /// SAFETY: defers completely to `&RenderEntity` implementation,
    /// and then only modifies the output safely.
    unsafe impl WorldQuery for RenderEntity {
        type Fetch<'w> = <&'static RenderEntity as WorldQuery>::Fetch<'w>;
        type State = <&'static RenderEntity as WorldQuery>::State;

        fn shrink_fetch<'wlong: 'wshort, 'wshort>(
            fetch: Self::Fetch<'wlong>,
        ) -> Self::Fetch<'wshort> {
            fetch
        }

        #[inline]
        unsafe fn init_fetch<'w, 's>(
            world: UnsafeWorldCell<'w>,
            component_id: &'s ComponentId,
            last_run: Tick,
            this_run: Tick,
        ) -> Self::Fetch<'w> {
            // SAFETY: defers to the `&T` implementation, with T set to `RenderEntity`.
            unsafe {
                <&RenderEntity as WorldQuery>::init_fetch(world, component_id, last_run, this_run)
            }
        }

        const IS_DENSE: bool = <&'static RenderEntity as WorldQuery>::IS_DENSE;

        #[inline]
        unsafe fn set_archetype<'w, 's>(
            fetch: &mut Self::Fetch<'w>,
            component_id: &'s ComponentId,
            archetype: &'w Archetype,
            table: &'w Table,
        ) {
            // SAFETY: defers to the `&T` implementation, with T set to `RenderEntity`.
            unsafe {
                <&RenderEntity as WorldQuery>::set_archetype(fetch, component_id, archetype, table);
            }
        }

        #[inline]
        unsafe fn set_table<'w, 's>(
            fetch: &mut Self::Fetch<'w>,
            &component_id: &'s ComponentId,
            table: &'w Table,
        ) {
            // SAFETY: defers to the `&T` implementation, with T set to `RenderEntity`.
            unsafe { <&RenderEntity as WorldQuery>::set_table(fetch, &component_id, table) }
        }

        fn update_component_access(&component_id: &ComponentId, access: &mut FilteredAccess) {
            <&RenderEntity as WorldQuery>::update_component_access(&component_id, access);
        }

        fn init_state(world: &mut World) -> ComponentId {
            <&RenderEntity as WorldQuery>::init_state(world)
        }

        fn get_state(components: &Components) -> Option<Self::State> {
            <&RenderEntity as WorldQuery>::get_state(components)
        }

        fn matches_component_set(
            &state: &ComponentId,
            set_contains_id: &impl Fn(ComponentId) -> bool,
        ) -> bool {
            <&RenderEntity as WorldQuery>::matches_component_set(&state, set_contains_id)
        }
    }

    // SAFETY: Component access of Self::ReadOnly is a subset of Self.
    // Self::ReadOnly matches exactly the same archetypes/tables as Self.
    unsafe impl QueryData for RenderEntity {
        const IS_READ_ONLY: bool = true;
        type ReadOnly = RenderEntity;
        type Item<'w, 's> = Entity;

        fn shrink<'wlong: 'wshort, 'wshort, 's>(
            item: Self::Item<'wlong, 's>,
        ) -> Self::Item<'wshort, 's> {
            item
        }

        #[inline(always)]
        unsafe fn fetch<'w, 's>(
            state: &'s Self::State,
            fetch: &mut Self::Fetch<'w>,
            entity: Entity,
            table_row: TableRow,
        ) -> Self::Item<'w, 's> {
            // SAFETY: defers to the `&T` implementation, with T set to `RenderEntity`.
            let component =
                unsafe { <&RenderEntity as QueryData>::fetch(state, fetch, entity, table_row) };
            component.id()
        }
    }

    // SAFETY: the underlying `Entity` is copied, and no mutable access is provided.
    unsafe impl ReadOnlyQueryData for RenderEntity {}

    impl ReleaseStateQueryData for RenderEntity {
        fn release_state<'w>(item: Self::Item<'w, '_>) -> Self::Item<'w, 'static> {
            item
        }
    }

    /// SAFETY: defers completely to `&RenderEntity` implementation,
    /// and then only modifies the output safely.
    unsafe impl WorldQuery for MainEntity {
        type Fetch<'w> = <&'static MainEntity as WorldQuery>::Fetch<'w>;
        type State = <&'static MainEntity as WorldQuery>::State;

        fn shrink_fetch<'wlong: 'wshort, 'wshort>(
            fetch: Self::Fetch<'wlong>,
        ) -> Self::Fetch<'wshort> {
            fetch
        }

        #[inline]
        unsafe fn init_fetch<'w, 's>(
            world: UnsafeWorldCell<'w>,
            component_id: &'s ComponentId,
            last_run: Tick,
            this_run: Tick,
        ) -> Self::Fetch<'w> {
            // SAFETY: defers to the `&T` implementation, with T set to `MainEntity`.
            unsafe {
                <&MainEntity as WorldQuery>::init_fetch(world, component_id, last_run, this_run)
            }
        }

        const IS_DENSE: bool = <&'static MainEntity as WorldQuery>::IS_DENSE;

        #[inline]
        unsafe fn set_archetype<'w, 's>(
            fetch: &mut Self::Fetch<'w>,
            component_id: &ComponentId,
            archetype: &'w Archetype,
            table: &'w Table,
        ) {
            // SAFETY: defers to the `&T` implementation, with T set to `MainEntity`.
            unsafe {
                <&MainEntity as WorldQuery>::set_archetype(fetch, component_id, archetype, table);
            }
        }

        #[inline]
        unsafe fn set_table<'w, 's>(
            fetch: &mut Self::Fetch<'w>,
            &component_id: &'s ComponentId,
            table: &'w Table,
        ) {
            // SAFETY: defers to the `&T` implementation, with T set to `MainEntity`.
            unsafe { <&MainEntity as WorldQuery>::set_table(fetch, &component_id, table) }
        }

        fn update_component_access(&component_id: &ComponentId, access: &mut FilteredAccess) {
            <&MainEntity as WorldQuery>::update_component_access(&component_id, access);
        }

        fn init_state(world: &mut World) -> ComponentId {
            <&MainEntity as WorldQuery>::init_state(world)
        }

        fn get_state(components: &Components) -> Option<Self::State> {
            <&MainEntity as WorldQuery>::get_state(components)
        }

        fn matches_component_set(
            &state: &ComponentId,
            set_contains_id: &impl Fn(ComponentId) -> bool,
        ) -> bool {
            <&MainEntity as WorldQuery>::matches_component_set(&state, set_contains_id)
        }
    }

    // SAFETY: Component access of Self::ReadOnly is a subset of Self.
    // Self::ReadOnly matches exactly the same archetypes/tables as Self.
    unsafe impl QueryData for MainEntity {
        const IS_READ_ONLY: bool = true;
        type ReadOnly = MainEntity;
        type Item<'w, 's> = Entity;

        fn shrink<'wlong: 'wshort, 'wshort, 's>(
            item: Self::Item<'wlong, 's>,
        ) -> Self::Item<'wshort, 's> {
            item
        }

        #[inline(always)]
        unsafe fn fetch<'w, 's>(
            state: &'s Self::State,
            fetch: &mut Self::Fetch<'w>,
            entity: Entity,
            table_row: TableRow,
        ) -> Self::Item<'w, 's> {
            // SAFETY: defers to the `&T` implementation, with T set to `MainEntity`.
            let component =
                unsafe { <&MainEntity as QueryData>::fetch(state, fetch, entity, table_row) };
            component.id()
        }
    }

    // SAFETY: the underlying `Entity` is copied, and no mutable access is provided.
    unsafe impl ReadOnlyQueryData for MainEntity {}

    impl ReleaseStateQueryData for MainEntity {
        fn release_state<'w>(item: Self::Item<'w, '_>) -> Self::Item<'w, 'static> {
            item
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        component::Component,
        entity::Entity,
        lifecycle::{Add, Remove},
        observer::On,
        query::With,
        system::{Query, ResMut},
        world::World,
    };

    use super::{
        entity_sync_system, EntityRecord, MainEntity, PendingSyncEntity, RenderEntity,
        SyncToRenderWorld,
    };

    #[derive(Component)]
    struct RenderDataComponent;

    #[test]
    fn sync_world() {
        let mut main_world = World::new();
        let mut render_world = World::new();
        main_world.init_resource::<PendingSyncEntity>();

        main_world.add_observer(
            |trigger: On<Add, SyncToRenderWorld>, mut pending: ResMut<PendingSyncEntity>| {
                pending.push(EntityRecord::Added(trigger.target()));
            },
        );
        main_world.add_observer(
            |trigger: On<Remove, SyncToRenderWorld>,
             mut pending: ResMut<PendingSyncEntity>,
             query: Query<&RenderEntity>| {
                if let Ok(e) = query.get(trigger.target()) {
                    pending.push(EntityRecord::Removed(*e));
                };
            },
        );

        // spawn some empty entities for test
        for _ in 0..99 {
            main_world.spawn_empty();
        }

        // spawn
        let main_entity = main_world
            .spawn(RenderDataComponent)
            // indicates that its entity needs to be synchronized to the render world
            .insert(SyncToRenderWorld)
            .id();

        entity_sync_system(&mut main_world, &mut render_world);

        let mut q = render_world.query_filtered::<Entity, With<MainEntity>>();

        // Only one synchronized entity
        assert!(q.iter(&render_world).count() == 1);

        let render_entity = q.single(&render_world).unwrap();
        let render_entity_component = main_world.get::<RenderEntity>(main_entity).unwrap();

        assert!(render_entity_component.id() == render_entity);

        let main_entity_component = render_world
            .get::<MainEntity>(render_entity_component.id())
            .unwrap();

        assert!(main_entity_component.id() == main_entity);

        // despawn
        main_world.despawn(main_entity);

        entity_sync_system(&mut main_world, &mut render_world);

        // Only one synchronized entity
        assert!(q.iter(&render_world).count() == 0);
    }
}
