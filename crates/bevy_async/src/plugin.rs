use crate::ecs_access::EcsAccess;
use crate::system_state_store::SystemStateStore;
use bevy_app::App;
use bevy_ecs::system::SystemParam;
use std::marker::PhantomData;
use std::sync::Arc;

/// Plugin entry point for the async <-> ECS bridge system.
///
/// This plugin installs a configuration resource telling the bridge how aggressively to drive work
/// at each sync point.
///
/// Conceptually, async tasks cannot directly access Bevy ECS state from arbitrary
/// threads or arbitrary times. Instead, they enqueue requests which are later
/// driven from a known ECS `SyncPoint` on the world-owning thread.
///
/// This supports arbitrary async runtimes as well as multiple Bevy Worlds / Bevy Apps.
pub struct AsyncPlugin {
    /// Upper bound on how many internal bridge ticks we perform each time a
    /// sync point system runs.
    ///
    /// A single "bridge tick" means:
    /// 1. collect queued access requests for that sync point,
    /// 2. wake the corresponding async tasks,
    /// 3. wait for each one to attempt a poll,
    /// 4. apply any deferred `SystemState` work back into the world.
    ///
    /// We may need to do this multiple times because one task's progress can
    /// unblock another task that previously returned `Poll::Pending`.
    pub max_async_ticks_per_sync_point: usize,
}

impl Default for AsyncPlugin {
    fn default() -> Self {
        Self {
            max_async_ticks_per_sync_point: 100,
        }
    }
}

impl bevy_app::Plugin for AsyncPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MaxAsyncTicksPerSyncPoint(
            self.max_async_ticks_per_sync_point,
        ))
        .init_resource::<AsyncBridge>();
    }
}

/// Internal resource to manage a limit on how many times we try to drive the async <-> ecs bridge
/// per sync point.
#[derive(bevy_ecs_macros::Resource, Clone)]
pub(crate) struct MaxAsyncTicksPerSyncPoint(pub(crate) usize);

/// This resource gives one the ability to create a bridge between an async task and the ecs.
/// By calling `AsyncBridge::new(&self)` you create a new bridge between an async task
/// and the ecs.
#[derive(bevy_ecs_macros::Resource, Default, Clone)]
pub struct AsyncBridge(pub(crate) Arc<crate::async_bridge::AsyncBridgeInner>);

impl AsyncBridge {
    /// Creates a reusable async handle for accessing the ECS with the
    /// `SystemParam` type `P`.
    ///
    /// This is the entry-point to let an
    /// async task interact with Bevy ECS state.
    ///
    /// The returned [`EcsAccess<P>`]:
    /// - is cheap to clone,
    /// - can be moved into async tasks,
    /// - does not access the world immediately,
    /// [`EcsAccess<P>`] waits until a matching sync point drives the bridge and
    ///   temporarily grants safe ECS access.
    ///
    /// You create one of these from a cloned [`AsyncBridge`] resource and
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
    ///   app.add_systems(Update, drive_async_bridge::<MySyncPoint>);
    ///   app.add_systems(Startup, move |bridge: Res<AsyncBridge>| {
    ///       let bridge = bridge.clone();
    ///       AsyncComputeTaskPool::get().spawn(async move {
    ///           let ecs_access = bridge.new::<Commands>();
    ///           ecs_access.access(MySyncPoint, |mut commands: Commands| {
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
    pub fn new<P: SystemParam + 'static>(&self) -> EcsAccess<P> {
        EcsAccess {
            phantom_data: PhantomData::default(),
            bridge: Arc::downgrade(&self.0),
            system_state: Arc::new(SystemStateStore::<P>::default()),
        }
    }
}
