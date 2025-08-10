//! Defines the [`AssetChanged`] query filter.
//!
//! Like [`Changed`](bevy_ecs::prelude::Changed), but for [`Asset`]s,
//! and triggers whenever the handle or the underlying asset changes.

use crate::{AsAssetId, Asset, AssetId};
use bevy_ecs::component::Components;
use bevy_ecs::{
    archetype::Archetype,
    component::{ComponentId, Tick},
    prelude::{Entity, Resource, World},
    query::{FilteredAccess, QueryData, QueryFilter, ReadFetch, WorldQuery},
    storage::{Table, TableRow},
    world::unsafe_world_cell::UnsafeWorldCell,
};
use bevy_platform::collections::HashMap;
use core::marker::PhantomData;
use disqualified::ShortName;
use tracing::error;

/// A resource that stores the last tick an asset was changed. This is used by
/// the [`AssetChanged`] filter to determine if an asset has changed since the last time
/// a query ran.
///
/// This resource is automatically managed by the [`AssetEventSystems`](crate::AssetEventSystems)
/// system set and should not be exposed to the user in order to maintain safety guarantees.
/// Any additional uses of this resource should be carefully audited to ensure that they do not
/// introduce any safety issues.
#[derive(Resource)]
pub(crate) struct AssetChanges<A: Asset> {
    change_ticks: HashMap<AssetId<A>, Tick>,
    last_change_tick: Tick,
}

impl<A: Asset> AssetChanges<A> {
    pub(crate) fn insert(&mut self, asset_id: AssetId<A>, tick: Tick) {
        self.last_change_tick = tick;
        self.change_ticks.insert(asset_id, tick);
    }
    pub(crate) fn remove(&mut self, asset_id: &AssetId<A>) {
        self.change_ticks.remove(asset_id);
    }
}

impl<A: Asset> Default for AssetChanges<A> {
    fn default() -> Self {
        Self {
            change_ticks: Default::default(),
            last_change_tick: Tick::new(0),
        }
    }
}

struct AssetChangeCheck<'w, A: AsAssetId> {
    // This should never be `None` in practice, but we need to handle the case
    // where the `AssetChanges` resource was removed.
    change_ticks: Option<&'w HashMap<AssetId<A::Asset>, Tick>>,
    last_run: Tick,
    this_run: Tick,
}

impl<A: AsAssetId> Clone for AssetChangeCheck<'_, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A: AsAssetId> Copy for AssetChangeCheck<'_, A> {}

impl<'w, A: AsAssetId> AssetChangeCheck<'w, A> {
    fn new(changes: &'w AssetChanges<A::Asset>, last_run: Tick, this_run: Tick) -> Self {
        Self {
            change_ticks: Some(&changes.change_ticks),
            last_run,
            this_run,
        }
    }
    // TODO(perf): some sort of caching? Each check has two levels of indirection,
    // which is not optimal.
    fn has_changed(&self, handle: &A) -> bool {
        let is_newer = |tick: &Tick| tick.is_newer_than(self.last_run, self.this_run);
        let id = handle.as_asset_id();

        self.change_ticks
            .is_some_and(|change_ticks| change_ticks.get(&id).is_some_and(is_newer))
    }
}

/// Filter that selects entities with an `A` for an asset that changed
/// after the system last ran, where `A` is a component that implements
/// [`AsAssetId`].
///
/// Unlike `Changed<A>`, this is true whenever the asset for the `A`
/// in `ResMut<Assets<A>>` changed. For example, when a mesh changed through the
/// [`Assets<Mesh>::get_mut`] method, `AssetChanged<Mesh>` will iterate over all
/// entities with the `Handle<Mesh>` for that mesh. Meanwhile, `Changed<Handle<Mesh>>`
/// will iterate over no entities.
///
/// Swapping the actual `A` component is a common pattern. So you
/// should check for _both_ `AssetChanged<A>` and `Changed<A>` with
/// `Or<(Changed<A>, AssetChanged<A>)>`.
///
/// # Quirks
///
/// - Asset changes are registered in the [`AssetEventSystems`] system set.
/// - Removed assets are not detected.
///
/// The list of changed assets only gets updated in the [`AssetEventSystems`] system set,
/// which runs in `PostUpdate`. Therefore, `AssetChanged` will only pick up asset changes in schedules
/// following [`AssetEventSystems`] or the next frame. Consider adding the system in the `Last` schedule
/// after [`AssetEventSystems`] if you need to react without frame delay to asset changes.
///
/// # Performance
///
/// When at least one `A` is updated, this will
/// read a hashmap once per entity with an `A` component. The
/// runtime of the query is proportional to how many entities with an `A`
/// it matches.
///
/// If no `A` asset updated since the last time the system ran, then no lookups occur.
///
/// [`AssetEventSystems`]: crate::AssetEventSystems
/// [`Assets<Mesh>::get_mut`]: crate::Assets::get_mut
pub struct AssetChanged<A: AsAssetId>(PhantomData<A>);

/// [`WorldQuery`] fetch for [`AssetChanged`].
#[doc(hidden)]
pub struct AssetChangedFetch<'w, A: AsAssetId> {
    inner: Option<ReadFetch<'w, A>>,
    check: AssetChangeCheck<'w, A>,
}

impl<'w, A: AsAssetId> Clone for AssetChangedFetch<'w, A> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            check: self.check,
        }
    }
}

/// [`WorldQuery`] state for [`AssetChanged`].
#[doc(hidden)]
pub struct AssetChangedState<A: AsAssetId> {
    asset_id: ComponentId,
    resource_id: ComponentId,
    _asset: PhantomData<fn(A)>,
}

#[expect(unsafe_code, reason = "WorldQuery is an unsafe trait.")]
/// SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
unsafe impl<A: AsAssetId> WorldQuery for AssetChanged<A> {
    type Fetch<'w> = AssetChangedFetch<'w, A>;

    type State = AssetChangedState<A>;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    unsafe fn init_fetch<'w, 's>(
        world: UnsafeWorldCell<'w>,
        state: &'s Self::State,
        last_run: Tick,
        this_run: Tick,
    ) -> Self::Fetch<'w> {
        // SAFETY:
        // - `AssetChanges` is private and only accessed mutably in the `AssetEventSystems` system set.
        // - `resource_id` was obtained from the type ID of `AssetChanges<A::Asset>`.
        let Some(changes) = (unsafe {
            world
                .get_resource_by_id(state.resource_id)
                .map(|ptr| ptr.deref::<AssetChanges<A::Asset>>())
        }) else {
            error!(
                "AssetChanges<{ty}> resource was removed, please do not remove \
                AssetChanges<{ty}> when using the AssetChanged<{ty}> world query",
                ty = ShortName::of::<A>()
            );

            return AssetChangedFetch {
                inner: None,
                check: AssetChangeCheck {
                    change_ticks: None,
                    last_run,
                    this_run,
                },
            };
        };
        let has_updates = changes.last_change_tick.is_newer_than(last_run, this_run);

        AssetChangedFetch {
            inner: has_updates.then(||
                    // SAFETY: We delegate to the inner `init_fetch` for `A`
                    unsafe {
                        <&A>::init_fetch(world, &state.asset_id, last_run, this_run)
                    }),
            check: AssetChangeCheck::new(changes, last_run, this_run),
        }
    }

    const IS_DENSE: bool = <&A>::IS_DENSE;

    unsafe fn set_archetype<'w, 's>(
        fetch: &mut Self::Fetch<'w>,
        state: &'s Self::State,
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        if let Some(inner) = &mut fetch.inner {
            // SAFETY: We delegate to the inner `set_archetype` for `A`
            unsafe {
                <&A>::set_archetype(inner, &state.asset_id, archetype, table);
            }
        }
    }

    unsafe fn set_table<'w, 's>(
        fetch: &mut Self::Fetch<'w>,
        state: &Self::State,
        table: &'w Table,
    ) {
        if let Some(inner) = &mut fetch.inner {
            // SAFETY: We delegate to the inner `set_table` for `A`
            unsafe {
                <&A>::set_table(inner, &state.asset_id, table);
            }
        }
    }

    #[inline]
    fn update_component_access(state: &Self::State, access: &mut FilteredAccess) {
        <&A>::update_component_access(&state.asset_id, access);
        access.add_resource_read(state.resource_id);
    }

    fn init_state(world: &mut World) -> AssetChangedState<A> {
        let resource_id = world.init_resource::<AssetChanges<A::Asset>>();
        let asset_id = world.register_component::<A>();
        AssetChangedState {
            asset_id,
            resource_id,
            _asset: PhantomData,
        }
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        let resource_id = components.resource_id::<AssetChanges<A::Asset>>()?;
        let asset_id = components.component_id::<A>()?;
        Some(AssetChangedState {
            asset_id,
            resource_id,
            _asset: PhantomData,
        })
    }

    fn matches_component_set(
        state: &Self::State,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(state.asset_id)
    }
}

#[expect(unsafe_code, reason = "QueryFilter is an unsafe trait.")]
/// SAFETY: read-only access
unsafe impl<A: AsAssetId> QueryFilter for AssetChanged<A> {
    const IS_ARCHETYPAL: bool = false;

    #[inline]
    unsafe fn filter_fetch(
        state: &Self::State,
        fetch: &mut Self::Fetch<'_>,
        entity: Entity,
        table_row: TableRow,
    ) -> bool {
        fetch.inner.as_mut().is_some_and(|inner| {
            // SAFETY: We delegate to the inner `fetch` for `A`
            unsafe {
                let handle = <&A>::fetch(&state.asset_id, inner, entity, table_row);
                fetch.check.has_changed(handle)
            }
        })
    }
}

#[cfg(test)]
#[expect(clippy::print_stdout, reason = "Allowed in tests.")]
mod tests {
    use crate::{AssetEventSystems, AssetPlugin, Handle};
    use alloc::{vec, vec::Vec};
    use core::num::NonZero;
    use std::println;

    use crate::{AssetApp, Assets};
    use bevy_app::{App, AppExit, PostUpdate, Startup, TaskPoolPlugin, Update};
    use bevy_ecs::schedule::IntoScheduleConfigs;
    use bevy_ecs::{
        component::Component,
        event::EventWriter,
        resource::Resource,
        system::{Commands, IntoSystem, Local, Query, Res, ResMut},
    };
    use bevy_reflect::TypePath;

    use super::*;

    #[derive(Asset, TypePath, Debug)]
    struct MyAsset(usize, &'static str);

    #[derive(Component)]
    struct MyComponent(Handle<MyAsset>);

    impl AsAssetId for MyComponent {
        type Asset = MyAsset;

        fn as_asset_id(&self) -> AssetId<Self::Asset> {
            self.0.id()
        }
    }

    fn run_app<Marker>(system: impl IntoSystem<(), (), Marker>) {
        let mut app = App::new();
        app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()))
            .init_asset::<MyAsset>()
            .add_systems(Update, system);
        app.update();
    }

    // According to a comment in QueryState::new in bevy_ecs, components on filter
    // position shouldn't conflict with components on query position.
    #[test]
    fn handle_filter_pos_ok() {
        fn compatible_filter(
            _query: Query<&mut MyComponent, AssetChanged<MyComponent>>,
            mut exit: EventWriter<AppExit>,
        ) {
            exit.write(AppExit::Error(NonZero::<u8>::MIN));
        }
        run_app(compatible_filter);
    }

    #[derive(Default, PartialEq, Debug, Resource)]
    struct Counter(Vec<u32>);

    fn count_update(
        mut counter: ResMut<Counter>,
        assets: Res<Assets<MyAsset>>,
        query: Query<&MyComponent, AssetChanged<MyComponent>>,
    ) {
        for handle in query.iter() {
            let asset = assets.get(&handle.0).unwrap();
            counter.0[asset.0] += 1;
        }
    }

    fn update_some(mut assets: ResMut<Assets<MyAsset>>, mut run_count: Local<u32>) {
        let mut update_index = |i| {
            let id = assets
                .iter()
                .find_map(|(h, a)| (a.0 == i).then_some(h))
                .unwrap();
            let asset = assets.get_mut(id).unwrap();
            println!("setting new value for {}", asset.0);
            asset.1 = "new_value";
        };
        match *run_count {
            0 | 1 => update_index(0),
            2 => {}
            3 => {
                update_index(0);
                update_index(1);
            }
            4.. => update_index(1),
        };
        *run_count += 1;
    }

    fn add_some(
        mut assets: ResMut<Assets<MyAsset>>,
        mut cmds: Commands,
        mut run_count: Local<u32>,
    ) {
        match *run_count {
            1 => {
                cmds.spawn(MyComponent(assets.add(MyAsset(0, "init"))));
            }
            0 | 2 => {}
            3 => {
                cmds.spawn(MyComponent(assets.add(MyAsset(1, "init"))));
                cmds.spawn(MyComponent(assets.add(MyAsset(2, "init"))));
            }
            4.. => {
                cmds.spawn(MyComponent(assets.add(MyAsset(3, "init"))));
            }
        };
        *run_count += 1;
    }

    #[track_caller]
    fn assert_counter(app: &App, assert: Counter) {
        assert_eq!(&assert, app.world().resource::<Counter>());
    }

    #[test]
    fn added() {
        let mut app = App::new();

        app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()))
            .init_asset::<MyAsset>()
            .insert_resource(Counter(vec![0, 0, 0, 0]))
            .add_systems(Update, add_some)
            .add_systems(PostUpdate, count_update.after(AssetEventSystems));

        // First run of the app, `add_systems(Startup…)` runs.
        app.update(); // run_count == 0
        assert_counter(&app, Counter(vec![0, 0, 0, 0]));
        app.update(); // run_count == 1
        assert_counter(&app, Counter(vec![1, 0, 0, 0]));
        app.update(); // run_count == 2
        assert_counter(&app, Counter(vec![1, 0, 0, 0]));
        app.update(); // run_count == 3
        assert_counter(&app, Counter(vec![1, 1, 1, 0]));
        app.update(); // run_count == 4
        assert_counter(&app, Counter(vec![1, 1, 1, 1]));
    }

    #[test]
    fn changed() {
        let mut app = App::new();

        app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()))
            .init_asset::<MyAsset>()
            .insert_resource(Counter(vec![0, 0]))
            .add_systems(
                Startup,
                |mut cmds: Commands, mut assets: ResMut<Assets<MyAsset>>| {
                    let asset0 = assets.add(MyAsset(0, "init"));
                    let asset1 = assets.add(MyAsset(1, "init"));
                    cmds.spawn(MyComponent(asset0.clone()));
                    cmds.spawn(MyComponent(asset0));
                    cmds.spawn(MyComponent(asset1.clone()));
                    cmds.spawn(MyComponent(asset1.clone()));
                    cmds.spawn(MyComponent(asset1));
                },
            )
            .add_systems(Update, update_some)
            .add_systems(PostUpdate, count_update.after(AssetEventSystems));

        // First run of the app, `add_systems(Startup…)` runs.
        app.update(); // run_count == 0

        // First run: We count the entities that were added in the `Startup` schedule
        assert_counter(&app, Counter(vec![2, 3]));

        // Second run: `update_once` updates the first asset, which is
        // associated with two entities, so `count_update` picks up two updates
        app.update(); // run_count == 1
        assert_counter(&app, Counter(vec![4, 3]));

        // Third run: `update_once` doesn't update anything, same values as last
        app.update(); // run_count == 2
        assert_counter(&app, Counter(vec![4, 3]));

        // Fourth run: We update the two assets (asset 0: 2 entities, asset 1: 3)
        app.update(); // run_count == 3
        assert_counter(&app, Counter(vec![6, 6]));

        // Fifth run: only update second asset
        app.update(); // run_count == 4
        assert_counter(&app, Counter(vec![6, 9]));
        // ibid
        app.update(); // run_count == 5
        assert_counter(&app, Counter(vec![6, 12]));
    }
}
