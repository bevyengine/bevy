use core::marker::PhantomData;

use bevy_app::{AppLabel, /*InternedAppLabel,*/ Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::{ContainsEntity, Entity, EntityEquivalent, EntityHash},
    lifecycle::{Add, Remove},
    observer::On,
    query::With,
    reflect::ReflectComponent,
    resource::Resource,
    system::{Local, Query, ResMut, SystemState},
    world::{EntityWorldMut, Mut, World},
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// A plugin that synchronizes entities with [`SyncToSubWorld`] between the main world and the sub world.
///
/// All entities with the [`SyncToSubWorld`] component are kept in sync. It
/// is automatically added as a required component by [`ExtractBaseComponentPlugin`]
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
/// between the main world and the sub world.
/// It does so by spawning and despawning entities in the sub world, to match spawned and despawned entities in the main world.
/// The link between synced entities is maintained by the [`SubEntity`] and [`MainEntity`] components.
///
/// The [`SubEntity`] contains the corresponding sub world entity of a main world entity, while [`MainEntity`] contains
/// the corresponding main world entity of a sub world entity.
/// For convenience, [`QueryData`](bevy_ecs::query::QueryData) implementations are provided for both components:
/// adding [`MainEntity`] to a query (without a `&`) will return the corresponding main world [`Entity`],
/// and adding [`SubEntity`] will return the corresponding sub world [`Entity`].
/// If you have access to the component itself, the underlying entities can be accessed by calling `.id()`.
///
/// Synchronization is necessary preparation for extraction ([`ExtractSchedule`](crate::ExtractSchedule)), which copies over component data from the main
/// to the sub world for these entities.
///
/// ```text
/// |--------------------------------------------------------------------|
/// |      |         |          Main world update                        |
/// | sync | extract |---------------------------------------------------|
/// |      |         |           Sub world update                        |
/// |--------------------------------------------------------------------|
/// ```
///
/// An example for synchronized main entities 1v1 and 18v1
///
/// ```text
/// |---------------------------Main World------------------------------|
/// |  Entity  |                    Component                           |
/// |-------------------------------------------------------------------|
/// | ID: 1v1  | PointLight | SubEntity(ID: 3V1) | SyncToSubWorld |
/// | ID: 18v1 | PointLight | SubEntity(ID: 5V1) | SyncToSubWorld |
/// |-------------------------------------------------------------------|
///
/// |----------Sub World--------------|
/// |  Entity  |       Component      |
/// |---------------------------------|
/// | ID: 3v1  | MainEntity(ID: 1V1)  |
/// | ID: 5v1  | MainEntity(ID: 18V1) |
/// |---------------------------------|
///
/// ```
///
/// Note that this effectively establishes a link between the main world entity and the sub world entity.
/// Not every entity needs to be synchronized, however; only entities with the [`SyncToSubWorld`] component are synced.
/// Adding [`SyncToSubWorld`] to a main world component will establish such a link.
/// Once a synchronized main entity is despawned, its corresponding Sub Entity will be automatically
/// despawned in the next `sync`.
///
/// The sync step does not copy any of component data between worlds, since its often not necessary to transfer over all
/// the components of a main world entity.
/// The render world probably cares about a `Position` component, but not a `Velocity` component.
/// The extraction happens in its own step, independently from, and after synchronization.
///
/// Moreover, [`SyncWorldPlugin`] only synchronizes *entities*. [`RenderAsset`]s like meshes and textures are handled
/// differently.
///
/// [`PipelinedRenderingPlugin`]: https://docs.rs/bevy/latest/bevy/render/pipelined_rendering/struct.PipelinedRenderingPlugin.html
/// [`ExtractBaseComponentPlugin`]: crate::extract_base_component::ExtractBaseComponentPlugin
/// [`SyncComponentPlugin`]: crate::sync_component::SyncComponentPlugin
/// [`RenderAsset`]: https://docs.rs/bevy/latest/bevy/render/render_asset/trait.RenderAsset.html
pub struct SyncWorldPlugin<L: AppLabel + Default> {
    marker: PhantomData<L>,
    // /// The [`AppLabel`] of the [`SubApp`](bevy_app::SubApp) to set up with extraction.
    // app_label: InternedAppLabel,
}

impl<L: AppLabel + Default> Default for SyncWorldPlugin<L> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
            // app_label: L::default().intern(),
        }
    }
}

impl<L: AppLabel + Default + Clone + Eq + Copy> Plugin for SyncWorldPlugin<L> {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<PendingSyncEntity<L>>();
        app.add_observer(
            |add: On<Add, SyncToSubWorld<L>>, mut pending: ResMut<PendingSyncEntity<L>>| {
                pending.push(EntityRecord::<L>::Added(add.entity));
            },
        );
        app.add_observer(
            |remove: On<Remove, SyncToSubWorld<L>>,
             mut pending: ResMut<PendingSyncEntity<L>>,
             query: Query<&SubEntity<L>>| {
                if let Ok(e) = query.get(remove.entity) {
                    pending.push(EntityRecord::<L>::Removed(*e));
                };
            },
        );
    }
}
/// Marker component that indicates that its entity needs to be synchronized to the sub world.
///
/// This component is automatically added as a required component by [`ExtractBaseComponentPlugin`] and [`SyncComponentPlugin`].
/// For more information see [`SyncWorldPlugin`].
///
/// NOTE: This component should persist throughout the entity's entire lifecycle.
/// If this component is removed from its entity, the entity will be despawned.
///
/// [`ExtractBaseComponentPlugin`]: crate::extract_base_component::ExtractBaseComponentPlugin
/// [`SyncComponentPlugin`]: crate::sync_component::SyncComponentPlugin
#[derive(Component, Copy, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default, Clone)]
#[component(storage = "SparseSet")]
pub struct SyncToSubWorld<L: AppLabel + Default + Clone>(PhantomData<L>);

/// Component added on the main world entities that are synced to the sub world in order to keep track of the corresponding sub world entity.
///
/// Can also be used as a newtype wrapper for sub world entities.
#[derive(Component, Deref, Copy, Clone, Debug, Eq, Hash, PartialEq, Reflect)]
#[component(clone_behavior = Ignore)]
#[reflect(Component, Clone)]
pub struct SubEntity<L: AppLabel + Clone + Eq + Copy>(#[deref] Entity, PhantomData<L>);

impl<L: AppLabel + Default + Clone + Eq + Copy> SubEntity<L> {
    #[inline]
    pub fn id(&self) -> Entity {
        self.0
    }
}

impl<L: AppLabel + Default + Clone + Eq + Copy> From<Entity> for SubEntity<L> {
    fn from(entity: Entity) -> Self {
        SubEntity(entity, PhantomData)
    }
}

impl<L: AppLabel + Default + Clone + Eq + Copy> ContainsEntity for SubEntity<L> {
    fn entity(&self) -> Entity {
        self.id()
    }
}

// SAFETY: SubEntity is a newtype around Entity that derives its comparison traits.
unsafe impl<L: AppLabel + Default + Clone + Eq + Copy> EntityEquivalent for SubEntity<L> {}

/// Component added on the sub world entities to keep track of the corresponding main world entity.
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

// SAFETY: SubEntity is a newtype around Entity that derives its comparison traits.
unsafe impl EntityEquivalent for MainEntity {}

/// A [`HashMap`] pre-configured to use [`EntityHash`] hashing with a [`MainEntity`].
pub type MainEntityHashMap<V> = HashMap<MainEntity, V, EntityHash>;

/// A [`HashSet`] pre-configured to use [`EntityHash`] hashing with a [`MainEntity`]..
pub type MainEntityHashSet = HashSet<MainEntity, EntityHash>;

/// Marker component that indicates that its entity needs to be despawned at the end of the frame.
#[derive(Component, Copy, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct TemporarySubEntity;

/// A record enum to what entities with [`SyncToSubWorld`] have been added or removed.
#[derive(Debug)]
pub(crate) enum EntityRecord<L: AppLabel + Default + Clone + Eq + Copy> {
    /// When an entity is spawned on the main world, notify the sub world so that it can spawn a corresponding
    /// entity. This contains the main world entity.
    Added(Entity),
    /// When an entity is despawned on the main world, notify the sub world so that the corresponding entity can be
    /// despawned. This contains the sub world entity.
    Removed(SubEntity<L>),
    /// When a component is removed from an entity, notify the sub world so that the corresponding component can be
    /// removed. This contains the main world entity.
    ComponentRemoved(Entity, fn(EntityWorldMut<'_>)),
}

// Entity Record in MainWorld pending to Sync
#[derive(Resource, Default, Deref, DerefMut)]
pub(crate) struct PendingSyncEntity<L: AppLabel + Default + Clone + Eq + Copy> {
    #[deref]
    records: Vec<EntityRecord<L>>,
    marker: PhantomData<L>,
}

pub(crate) fn entity_sync_system<L: AppLabel + Default + Clone + Eq + Copy>(
    main_world: &mut World,
    sub_world: &mut World,
) {
    main_world.resource_scope(|world, mut pending: Mut<PendingSyncEntity<L>>| {
        // TODO : batching record
        for record in pending.drain(..) {
            match record {
                EntityRecord::Added(e) => {
                    if let Ok(mut main_entity) = world.get_entity_mut(e) {
                        match main_entity.entry::<SubEntity<L>>() {
                            bevy_ecs::world::ComponentEntry::Occupied(_) => {
                                panic!("Attempting to synchronize an entity that has already been synchronized!");
                            }
                            bevy_ecs::world::ComponentEntry::Vacant(entry) => {
                                let id = sub_world.spawn(MainEntity(e)).id();

                                entry.insert(SubEntity::<L>(id, PhantomData));
                            }
                        };
                    }
                }
                EntityRecord::Removed(sub_entity) => {
                    if let Ok(ec) = sub_world.get_entity_mut(sub_entity.id()) {
                        ec.despawn();
                    };
                }
                EntityRecord::ComponentRemoved(main_entity, removal_function) => {
                    let Some(sub_entity) = world.get::<SubEntity<L>>(main_entity) else {
                        continue;
                    };
                    if let Ok(sub_world_entity) = sub_world.get_entity_mut(sub_entity.id()) {
                        removal_function(sub_world_entity);
                    }
                },
            }
        }
    });
}

pub(crate) fn despawn_temporary_sub_entities(
    world: &mut World,
    state: &mut SystemState<Query<Entity, With<TemporarySubEntity>>>,
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
/// The implementations for both [`MainEntity`] and [`SubEntity`] should stay in sync,
/// and are based off of the `&T` implementation in `bevy_ecs`.
mod sub_entities_world_query_impls {
    use super::{MainEntity, SubEntity};

    use bevy_app::AppLabel;
    use bevy_ecs::{
        archetype::Archetype,
        change_detection::Tick,
        component::{ComponentId, Components},
        entity::Entity,
        query::{
            ArchetypeQueryData, FilteredAccess, IterQueryData, QueryData, ReadOnlyQueryData,
            ReleaseStateQueryData, SingleEntityQueryData, WorldQuery,
        },
        storage::{Table, TableRow},
        world::{unsafe_world_cell::UnsafeWorldCell, World},
    };

    // SAFETY: defers completely to `&SubEntity` implementation,
    // and then only modifies the output safely.
    unsafe impl<L: AppLabel + Default + Clone + Eq + Copy> WorldQuery for SubEntity<L> {
        type Fetch<'w> = <&'static SubEntity<L> as WorldQuery>::Fetch<'w>;
        type State = <&'static SubEntity<L> as WorldQuery>::State;

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
            // SAFETY: defers to the `&T` implementation, with T set to `SubEntity`.
            unsafe {
                <&SubEntity<L> as WorldQuery>::init_fetch(world, component_id, last_run, this_run)
            }
        }

        const IS_DENSE: bool = <&'static SubEntity<L> as WorldQuery>::IS_DENSE;

        #[inline]
        unsafe fn set_archetype<'w, 's>(
            fetch: &mut Self::Fetch<'w>,
            component_id: &'s ComponentId,
            archetype: &'w Archetype,
            table: &'w Table,
        ) {
            // SAFETY: defers to the `&T` implementation, with T set to `SubEntity`.
            unsafe {
                <&SubEntity<L> as WorldQuery>::set_archetype(fetch, component_id, archetype, table);
            }
        }

        #[inline]
        unsafe fn set_table<'w, 's>(
            fetch: &mut Self::Fetch<'w>,
            &component_id: &'s ComponentId,
            table: &'w Table,
        ) {
            // SAFETY: defers to the `&T` implementation, with T set to `SubEntity`.
            unsafe { <&SubEntity<L> as WorldQuery>::set_table(fetch, &component_id, table) }
        }

        fn update_component_access(&component_id: &ComponentId, access: &mut FilteredAccess) {
            <&SubEntity<L> as WorldQuery>::update_component_access(&component_id, access);
        }

        fn init_state(world: &mut World) -> ComponentId {
            <&SubEntity<L> as WorldQuery>::init_state(world)
        }

        fn get_state(components: &Components) -> Option<Self::State> {
            <&SubEntity<L> as WorldQuery>::get_state(components)
        }

        fn matches_component_set(
            &state: &ComponentId,
            set_contains_id: &impl Fn(ComponentId) -> bool,
        ) -> bool {
            <&SubEntity<L> as WorldQuery>::matches_component_set(&state, set_contains_id)
        }
    }

    // SAFETY: Component access of Self::ReadOnly is a subset of Self.
    // Self::ReadOnly matches exactly the same archetypes/tables as Self.
    unsafe impl<L: AppLabel + Default + Clone + Eq + Copy> QueryData for SubEntity<L> {
        const IS_READ_ONLY: bool = true;
        const IS_ARCHETYPAL: bool = <&MainEntity as QueryData>::IS_ARCHETYPAL;
        type ReadOnly = SubEntity<L>;
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
        ) -> Option<Self::Item<'w, 's>> {
            // SAFETY: defers to the `&T` implementation, with T set to `SubEntity`.
            let component =
                unsafe { <&SubEntity<L> as QueryData>::fetch(state, fetch, entity, table_row) };
            component.map(SubEntity::id)
        }

        fn iter_access(
            state: &Self::State,
        ) -> impl Iterator<Item = bevy_ecs::query::EcsAccessType<'_>> {
            <&SubEntity<L> as QueryData>::iter_access(state)
        }
    }

    /// SAFETY: access is read only and only on the current entity
    unsafe impl<L: AppLabel + Default + Clone + Eq + Copy> IterQueryData for SubEntity<L> {}

    /// SAFETY: access is read only
    unsafe impl<L: AppLabel + Default + Clone + Eq + Copy> ReadOnlyQueryData for SubEntity<L> {}

    /// SAFETY: access is only on the current entity
    unsafe impl<L: AppLabel + Default + Clone + Eq + Copy> SingleEntityQueryData for SubEntity<L> {}

    impl<L: AppLabel + Default + Clone + Eq + Copy> ArchetypeQueryData for SubEntity<L> {}

    impl<L: AppLabel + Default + Clone + Eq + Copy> ReleaseStateQueryData for SubEntity<L> {
        fn release_state<'w>(item: Self::Item<'w, '_>) -> Self::Item<'w, 'static> {
            item
        }
    }

    // SAFETY: defers completely to `&SubEntity` implementation,
    // and then only modifies the output safely.
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
        const IS_ARCHETYPAL: bool = <&MainEntity as QueryData>::IS_ARCHETYPAL;
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
        ) -> Option<Self::Item<'w, 's>> {
            // SAFETY: defers to the `&T` implementation, with T set to `MainEntity`.
            let component =
                unsafe { <&MainEntity as QueryData>::fetch(state, fetch, entity, table_row) };
            component.map(MainEntity::id)
        }

        fn iter_access(
            state: &Self::State,
        ) -> impl Iterator<Item = bevy_ecs::query::EcsAccessType<'_>> {
            <&MainEntity as QueryData>::iter_access(state)
        }
    }

    /// SAFETY: access is read only and only on the current entity
    unsafe impl IterQueryData for MainEntity {}

    /// SAFETY: access is read only
    unsafe impl ReadOnlyQueryData for MainEntity {}

    /// SAFETY: access is only on the current entity
    unsafe impl SingleEntityQueryData for MainEntity {}

    impl ArchetypeQueryData for MainEntity {}

    impl ReleaseStateQueryData for MainEntity {
        fn release_state<'w>(item: Self::Item<'w, '_>) -> Self::Item<'w, 'static> {
            item
        }
    }
}

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;

    use bevy_app::AppLabel;
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
        entity_sync_system, EntityRecord, MainEntity, PendingSyncEntity, SubEntity, SyncToSubWorld,
    };

    #[derive(Component)]
    struct RenderDataComponent;

    #[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
    struct ExtractApp;

    #[test]
    fn sync_world() {
        let mut main_world = World::new();
        let mut render_world = World::new();
        main_world.init_resource::<PendingSyncEntity<ExtractApp>>();

        main_world.add_observer(
            |add: On<Add, SyncToSubWorld<ExtractApp>>,
             mut pending: ResMut<PendingSyncEntity<ExtractApp>>| {
                pending.push(EntityRecord::Added(add.entity));
            },
        );
        main_world.add_observer(
            |remove: On<Remove, SyncToSubWorld<ExtractApp>>,
             mut pending: ResMut<PendingSyncEntity<ExtractApp>>,
             query: Query<&SubEntity<ExtractApp>>| {
                if let Ok(e) = query.get(remove.entity) {
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
            .insert(SyncToSubWorld::<ExtractApp>(PhantomData::<ExtractApp>))
            .id();

        entity_sync_system::<ExtractApp>(&mut main_world, &mut render_world);

        let mut q = render_world.query_filtered::<Entity, With<MainEntity>>();

        // Only one synchronized entity
        assert!(q.iter(&render_world).count() == 1);

        let render_entity = q.single(&render_world).unwrap();
        let render_entity_component = main_world
            .get::<SubEntity<ExtractApp>>(main_entity)
            .unwrap();

        assert!(render_entity_component.id() == render_entity);

        let main_entity_component = render_world
            .get::<MainEntity>(render_entity_component.id())
            .unwrap();

        assert!(main_entity_component.id() == main_entity);

        // despawn
        main_world.despawn(main_entity);

        entity_sync_system::<ExtractApp>(&mut main_world, &mut render_world);

        // Only one synchronized entity
        assert!(q.iter(&render_world).count() == 0);
    }
}
