use crate::bridge_future::{AsyncSystemParamFunction, AsyncSystemState, BridgeFuture};
use bevy_app::App;
use bevy_ecs::system::SystemParam;
use bevy_platform::sync::{Arc, Weak};

/// Plugin entry point for the async <-> ECS bridge system.
///
/// Conceptually, async tasks cannot directly access Bevy ECS state from arbitrary
/// threads or arbitrary times. Instead, they enqueue requests which are later
/// driven from a known `SyncPoint` on the world-owning thread.
///
/// This supports arbitrary async runtimes as well as multiple Bevy Worlds / Bevy Apps.
///
/// To configure how "aggressively" sync points drive work, see [`AsyncTickBudget`].
#[derive(Default)]
pub struct AsyncPlugin;

impl bevy_app::Plugin for AsyncPlugin {
    fn build(&self, app: &mut App) {
        let strong_world = StrongAsyncWorld::default();
        let weak_world = AsyncWorld(Arc::downgrade(&strong_world.0));
        app.insert_resource(strong_world)
            .insert_resource(weak_world);
    }
}

/// This resource gives one the ability to create a bridge between an async task and the ecs.
/// By calling `AsyncBridge::new(&self)` you create a new bridge between an async task
/// and the ecs.
#[derive(bevy_ecs_macros::Resource, Default, Clone)]
pub struct AsyncWorld(pub(crate) Weak<crate::bridge_request::AsyncWorldInner>);

/// [`StrongAsyncWorld`] is the singular strong handle to the Inner that lives in a private Resource.
/// We only expose [`Weak`] handles publicly so we can rely on the behavior that if the `World`
/// is dropped then we can detect it by a failing [`Weak::upgrade`]
#[derive(bevy_ecs_macros::Resource, Default, Clone)]
pub(crate) struct StrongAsyncWorld(pub(crate) Arc<crate::bridge_request::AsyncWorldInner>);

impl AsyncWorld {
    /// Creates a reusable async handle for accessing the ECS with the
    /// `SystemParam` type `P`.
    ///
    /// This is the entry-point to let an
    /// async task interact with Bevy ECS state.
    ///
    /// The returned [`AsyncSystemState<P>`]:
    /// - is cheap to clone,
    /// - can be moved into async tasks,
    /// - does not access the world immediately,
    ///
    /// [`AsyncSystemState<P>`] waits until a matching sync point drives the bridge and
    /// temporarily grants safe ECS access.
    ///
    /// You create one of these from a cloned [`AsyncWorld`] resource and
    /// then call `.access(...)` inside async code whenever you want to access the ECS.
    ///
    /// # Example
    /// ```rust
    /// use bevy_app::prelude::*;
    /// use bevy_async::prelude::*;
    /// use bevy_ecs::prelude::*;
    /// use bevy_tasks::AsyncComputeTaskPool;
    /// use bevy_platform::sync::atomic::AtomicBool;
    /// use bevy_platform::sync::atomic::Ordering;
    /// use bevy_platform::sync::Arc;
    /// use bevy_app::ScheduleRunnerPlugin;
    ///
    /// struct MySyncPoint;
    /// static ACCESS_RAN: AtomicBool = AtomicBool::new(false);
    /// fn main() {
    ///   let mut app = App::new();
    ///   app.add_plugins((AsyncPlugin::default(), ScheduleRunnerPlugin::default(), TaskPoolPlugin::default()));
    ///   app.add_systems(Update, async_world_sync_point::<MySyncPoint>);
    ///   app.add_systems(Startup, move |world: Res<AsyncWorld>| {
    ///       let world = world.clone();
    ///       AsyncComputeTaskPool::get().spawn(async move {
    ///           let system_state = world.system_state::<Commands>();
    ///           system_state.bridge(MySyncPoint, |mut commands: Commands| {
    ///               commands.spawn_empty();
    ///               ACCESS_RAN.store(true, Ordering::Relaxed);
    ///           }).await.unwrap();
    ///       }).detach();
    ///   });
    ///   app.update();
    ///
    ///   assert!(ACCESS_RAN.load(Ordering::Relaxed));
    /// }
    ///
    /// ```
    ///
    /// `P` is stored lazily, meaning the underlying `SystemState<P>` is only
    /// initialized when the bridge is first driven against a real `World`.
    pub fn system_state<P: SystemParam + 'static>(&self) -> AsyncSystemState<P> {
        AsyncSystemState::new(self.clone())
    }

    /// Bridge into the ECS with the `SystemParam`s specified only by the closure's argument
    /// types (up to 16 of them), like a regular Bevy system:
    ///
    /// ```rust,ignore
    /// world.bridge(MySyncPoint, |mut commands: Commands, query: Query<&Health>| {
    ///     // ...
    /// }).await?;
    /// ```
    ///
    /// This creates a fresh [`AsyncSystemState`] for each call. If you want params like `Local`
    /// or filters like `Changed` to persist across calls, create one with
    /// [`AsyncWorld::system_state`] and reuse it instead.
    pub fn bridge<SyncPoint: 'static, Marker: 'static, BridgeFn>(
        &self,
        sync_point: SyncPoint,
        bridge_fn: BridgeFn,
    ) -> BridgeFuture<BridgeFn, Marker>
    where
        BridgeFn: AsyncSystemParamFunction<Marker>,
    {
        self.system_state::<BridgeFn::Param>()
            .bridge(sync_point, bridge_fn)
    }
}
