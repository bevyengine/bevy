//! Define the [`AssetChanged`] query filter.
//!
//! Like [`Changed`], but for [`Asset`]s.
use std::marker::PhantomData;

use bevy_ecs::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{ComponentId, Tick},
    prelude::{Changed, Entity, Or, Resource, World},
    query::{Access, FilteredAccess, QueryItem, ReadFetch, WorldQuery, WorldQueryFilter},
    storage::{Table, TableRow},
    world::unsafe_world_cell::UnsafeWorldCell,
};
use bevy_utils::{get_short_name, HashMap};

use crate::{Asset, AssetId, Handle};

#[doc(hidden)]
#[derive(Resource)]
pub struct AssetChanges<A: Asset> {
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

struct AssetChangeCheck<'w, A: Asset> {
    change_ticks: &'w HashMap<AssetId<A>, Tick>,
    last_run: Tick,
    this_run: Tick,
}

impl<A: Asset> Clone for AssetChangeCheck<'_, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A: Asset> Copy for AssetChangeCheck<'_, A> {}

impl<'w, A: Asset> AssetChangeCheck<'w, A> {
    fn new(changes: &'w AssetChanges<A>, last_run: Tick, this_run: Tick) -> Self {
        Self {
            change_ticks: &changes.change_ticks,
            last_run,
            this_run,
        }
    }
    // TODO(perf): some sort of caching? Each check has two levels of indirection,
    // which is not optimal.
    fn has_changed(&self, handle: &Handle<A>) -> bool {
        let is_newer = |tick: &Tick| tick.is_newer_than(self.last_run, self.this_run);
        let id = &handle.id();

        self.change_ticks.get(id).is_some_and(is_newer)
    }
}

/// A shortcut for the commonly used `Or<(Changed<Handle<A>>, AssetChanged<A>)>`
/// query filter.
///
/// If you want to react to changes to `Handle<A>`, you need to:
/// - Check if the `Handle<A>` was changed through a `Query<&mut Handle<A>>`.
/// - Check if the `A` in `Assets<A>` pointed by `Handle<A>` was changed through an
///   [`Assets<A>::get_mut`].
///
/// To properly handle both cases, you need to combine the `Changed` and `AssetChanged`
/// filters. This query filter is exactly this.
///
/// [`Assets<A>::get_mut`]: crate::Assets::get_mut
pub type AssetOrHandleChanged<A> = Or<(Changed<Handle<A>>, AssetChanged<A>)>;

/// Filter that selects entities with a `Handle<A>` for an asset that changed
/// after the system last ran.
///
/// Unlike `Changed<Handle<A>>`, this is true whenever the asset for the `Handle<A>`
/// in `ResMut<Assets<A>>` changed. For example, when a mesh changed through the
/// [`Assets<Mesh>::get_mut`] method, `AssetChanged<Mesh>` will iterate over all
/// entities with the `Handle<Mesh>` for that mesh. Meanwhile, `Changed<Handle<Mesh>>`
/// will iterate over no entities.
///
/// Swapping the actual `Handle<A>` component is a common pattern. So you
/// should check for _both_ `AssetChanged<A>` and `Changed<Handle<A>>` with
/// [`AssetOrHandleChanged`]
///
/// # Quirks
///
/// - Asset changes are registered in the [`AssetEvents`] schedule.
/// - Removed assets are not detected.
/// - Asset update tracking only starts after the first system with an
///   `AssetChanged` query parameter ran for the first time.
///
/// This means that assets added before the system ran won't be detected
/// (for example in a `Startup` schedule).
///
/// This is also true of systems gated by a `run_if` condition.
///
/// The list of changed assets only gets updated in the
/// [`AssetEvents`] schedule, just after `PostUpdate`. Therefore, `AssetChanged`
/// will only pick up asset changes in schedules following `AssetEvents` or the
/// next frame. Consider adding the system in the `Last` schedule if you need
/// to react without frame delay to asset changes.
///
/// # Performance
///
/// When at least one `A` is updated, this will
/// read a hashmap once per entity with a `Handle<A>` component. The
/// runtime of the query is proportional to how many entities with a `Handle<A>`
/// it matches.
///
/// If no `A` asset updated since the last time the system ran, then no lookups occur.
///
/// [`AssetEvents`]: crate::AssetEvents
/// [`Assets<Mesh>::get_mut`]: crate::Assets::get_mut
pub struct AssetChanged<A: Asset>(PhantomData<A>);

/// Fetch for [`AssetChanged`].
#[doc(hidden)]
pub struct AssetChangedFetch<'w, A: Asset> {
    inner: Option<ReadFetch<'w, Handle<A>>>,
    check: AssetChangeCheck<'w, A>,
}

impl<'w, A: Asset> Clone for AssetChangedFetch<'w, A> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            check: self.check,
        }
    }
}

/// State for [`AssetChanged`].
#[doc(hidden)]
pub struct AssetChangedState<A: Asset> {
    handle_id: ComponentId,
    resource_id: ComponentId,
    archetype_id: ArchetypeComponentId,
    _asset: PhantomData<fn(A)>,
}

// SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
unsafe impl<A: Asset> WorldQuery for AssetChanged<A> {
    type Fetch<'w> = AssetChangedFetch<'w, A>;
    type State = AssetChangedState<A>;

    type Item<'w> = ();

    fn shrink<'wlong: 'wshort, 'wshort>(_: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self> {}

    fn init_state(world: &mut World) -> AssetChangedState<A> {
        let resource_id = world.init_resource::<AssetChanges<A>>();
        let archetype_id = world.storages().resources.get(resource_id).unwrap().id();
        AssetChangedState {
            handle_id: world.init_component::<Handle<A>>(),
            resource_id,
            archetype_id,
            _asset: PhantomData,
        }
    }

    fn matches_component_set(
        state: &Self::State,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(state.handle_id)
    }

    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        state: &Self::State,
        last_run: Tick,
        this_run: Tick,
    ) -> Self::Fetch<'w> {
        let err_msg = || {
            panic!(
                "AssetChanges<{ty}> resource was removed, please do not remove \
                AssetChanges<{ty}> when using the AssetChanged<{ty}> world query",
                ty = get_short_name(std::any::type_name::<A>())
            )
        };
        let changes: &AssetChanges<_> = world.get_resource().unwrap_or_else(err_msg);
        let has_updates = changes.last_change_tick.is_newer_than(last_run, this_run);
        AssetChangedFetch {
            inner: has_updates
                .then(|| <&_>::init_fetch(world, &state.handle_id, last_run, this_run)),
            check: AssetChangeCheck::new(changes, last_run, this_run),
        }
    }

    const IS_DENSE: bool = <&Handle<A>>::IS_DENSE;

    unsafe fn set_table<'w>(fetch: &mut Self::Fetch<'w>, state: &Self::State, table: &'w Table) {
        if let Some(inner) = &mut fetch.inner {
            <&Handle<A>>::set_table(inner, &state.handle_id, table);
        }
    }

    unsafe fn set_archetype<'w>(
        fetch: &mut Self::Fetch<'w>,
        state: &Self::State,
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        if let Some(inner) = &mut fetch.inner {
            <&Handle<A>>::set_archetype(inner, &state.handle_id, archetype, table);
        }
    }

    unsafe fn fetch<'w>(_: &mut Self::Fetch<'w>, _: Entity, _: TableRow) -> Self::Item<'w> {}

    #[inline]
    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        <&Handle<A>>::update_component_access(&state.handle_id, access);
        access.add_read(state.resource_id);
    }

    #[inline]
    fn update_archetype_component_access(
        state: &Self::State,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        access.add_read(state.archetype_id);
        <&Handle<A>>::update_archetype_component_access(&state.handle_id, archetype, access);
    }
}

/// SAFETY: read-only access
impl<A: Asset> WorldQueryFilter for AssetChanged<A> {
    const IS_ARCHETYPAL: bool = false;

    #[inline]
    unsafe fn filter_fetch(
        fetch: &mut Self::Fetch<'_>,
        entity: Entity,
        table_row: TableRow,
    ) -> bool {
        fetch.inner.as_mut().map_or(false, |inner| {
            let handle = <&Handle<A>>::fetch(inner, entity, table_row);
            fetch.check.has_changed(handle)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as bevy_asset, AssetPlugin};

    use crate::{AssetApp, Assets};
    use bevy_app::{App, AppExit, Last, Startup, Update};
    use bevy_core::TaskPoolPlugin;
    use bevy_ecs::{
        event::EventWriter,
        system::{Commands, IntoSystem, Local, Query, Res, ResMut, Resource},
    };
    use bevy_reflect::TypePath;

    use super::*;

    #[derive(Asset, TypePath, Debug)]
    struct MyAsset(usize, &'static str);

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
            _query: Query<&mut Handle<MyAsset>, AssetChanged<MyAsset>>,
            mut exit: EventWriter<AppExit>,
        ) {
            exit.send(AppExit);
        }
        run_app(compatible_filter);
    }

    #[derive(Default, PartialEq, Debug, Resource)]
    struct Counter(Vec<u32>);

    fn count_update(
        mut counter: ResMut<Counter>,
        assets: Res<Assets<MyAsset>>,
        query: Query<&Handle<MyAsset>, AssetChanged<MyAsset>>,
    ) {
        for handle in query.iter() {
            let asset = assets.get(handle).unwrap();
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
                cmds.spawn(assets.add(MyAsset(0, "init")));
            }
            0 | 2 => {}
            3 => {
                cmds.spawn(assets.add(MyAsset(1, "init")));
                cmds.spawn(assets.add(MyAsset(2, "init")));
            }
            4.. => {
                cmds.spawn(assets.add(MyAsset(3, "init")));
            }
        };
        *run_count += 1;
    }

    #[track_caller]
    fn assert_counter(app: &App, assert: Counter) {
        assert_eq!(&assert, app.world.resource::<Counter>());
    }

    #[test]
    fn added() {
        let mut app = App::new();

        app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()))
            .init_asset::<MyAsset>()
            .insert_resource(Counter(vec![0, 0, 0, 0]))
            .add_systems(Update, add_some)
            .add_systems(Last, count_update);

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
                    cmds.spawn(asset0.clone());
                    cmds.spawn(asset0);
                    cmds.spawn(asset1.clone());
                    cmds.spawn(asset1.clone());
                    cmds.spawn(asset1);
                },
            )
            .add_systems(Update, update_some)
            .add_systems(Last, count_update);

        // First run of the app, `add_systems(Startup…)` runs.
        app.update(); // run_count == 0

        // `AssetChanges` do not react to `Added` or `Modified` events that occured before
        // the first run of `count_update`. This is why the counters are still 0
        assert_counter(&app, Counter(vec![0, 0]));

        // Second run: `update_once` updates the first asset, which is
        // associated with two entities, so `count_update` picks up two updates
        app.update(); // run_count == 1
        assert_counter(&app, Counter(vec![2, 0]));

        // Third run: `update_once` doesn't update anything, same values as last
        app.update(); // run_count == 2
        assert_counter(&app, Counter(vec![2, 0]));

        // Fourth run: We update the two assets (asset 0: 2 entities, asset 1: 3)
        app.update(); // run_count == 3
        assert_counter(&app, Counter(vec![4, 3]));

        // Fifth run: only update second asset
        app.update(); // run_count == 4
        assert_counter(&app, Counter(vec![4, 6]));
        // ibid
        app.update(); // run_count == 5
        assert_counter(&app, Counter(vec![4, 9]));
    }
}
