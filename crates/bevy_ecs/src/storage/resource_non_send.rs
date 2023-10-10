use std::any::TypeId;
use std::cell::RefCell;
use std::marker::PhantomData;

use bevy_ptr::{OwningPtr, Ptr};

use crate as bevy_ecs;
use crate::archetype::ArchetypeComponentId;
use crate::change_detection::{Mut, MutUntyped, TicksMut};
use crate::component::{ComponentId, Components, Tick, TickCells};
use crate::storage::{ResourceData, Resources};
use crate::system::{Resource, SystemParam};
use crate::world::{unsafe_world_cell::UnsafeWorldCell, World};

thread_local! {
    static TLS: RefCell<ThreadLocals> = RefCell::new(ThreadLocals::new());
}

/// A type that can be inserted into [`ThreadLocals`]. Unlike [`Resource`], this does not require
/// [`Send`] or [`Sync`].
pub trait ThreadLocalResource: 'static {}

/// Storage for registered [`ThreadLocalResource`] values.
pub struct ThreadLocals {
    info: Components,
    storage: Resources<false>,
    curr_tick: Tick,
    last_tick: Tick,
    // !Send + !Sync
    _marker: PhantomData<*const ()>,
}

impl ThreadLocals {
    /// Constructs a new instance of [`ThreadLocals`].
    pub(crate) fn new() -> Self {
        Self {
            info: Components::default(),
            storage: Resources::default(),
            curr_tick: Tick::new(0),
            last_tick: Tick::new(0),
            _marker: PhantomData,
        }
    }

    /// Initializes a new [`ThreadLocalResource`] type and returns the [`ComponentId`] created for
    /// it.
    pub fn init_resource_type<R: ThreadLocalResource>(&mut self) -> ComponentId {
        self.info.init_non_send::<R>()
    }

    /// Returns the [`ComponentId`] of the [`ThreadLocalResource`], if it exists.
    ///
    /// **Note:** The returned `ComponentId` is specific to this `ThreadLocals` instance. You should
    /// not use it with another `ThreadLocals` instance.
    #[inline]
    pub fn resource_id<R: ThreadLocalResource>(&self) -> Option<ComponentId> {
        self.info.get_resource_id(TypeId::of::<R>())
    }

    /// Inserts a new resource with its default value.
    ///
    /// If the resource already exists, nothing happens.
    #[inline]
    pub fn init_resource<R: ThreadLocalResource + Default>(&mut self) {
        if !self.contains_resource::<R>() {
            self.insert_resource::<R>(Default::default());
        }
    }

    /// Inserts a new resource with the given `value`.
    ///
    /// Resources are "unique" data of a given type. If you insert a resource of a type that already
    /// exists, you will overwrite any existing data.
    #[inline]
    pub fn insert_resource<R: ThreadLocalResource>(&mut self, value: R) {
        let id = self.init_resource_type::<R>();
        OwningPtr::make(value, |ptr| {
            // SAFETY: id was just initialized and corresponds to resource of type R
            unsafe {
                self.insert_resource_by_id(id, ptr);
            }
        });
    }

    /// Removes the resource of a given type and returns it, if it exists.
    #[inline]
    pub fn remove_resource<R: ThreadLocalResource>(&mut self) -> Option<R> {
        let id = self.info.get_resource_id(TypeId::of::<R>())?;
        let (ptr, _) = self.storage.get_mut(id)?.remove()?;
        // SAFETY: `id` came directly from `R`, so this has to be the `R` data
        unsafe { Some(ptr.read::<R>()) }
    }

    /// Returns `true` if a resource of type `R` exists.
    #[inline]
    pub fn contains_resource<R: ThreadLocalResource>(&self) -> bool {
        self.info
            .get_resource_id(TypeId::of::<R>())
            .and_then(|id| self.storage.get(id))
            .map(|info| info.is_present())
            .unwrap_or(false)
    }

    /// Return's `true` if a resource of type `R` has been added since the storage was last changed.
    pub fn is_resource_added<R: ThreadLocalResource>(&self) -> bool {
        self.info
            .get_resource_id(TypeId::of::<R>())
            .and_then(|id| self.storage.get(id)?.get_ticks())
            .map(|ticks| ticks.is_added(self.last_tick, self.curr_tick))
            .unwrap_or(false)
    }

    /// Return's `true` if a resource of type `R` has been added or mutably accessed since the
    /// storage was last changed.
    pub fn is_resource_changed<R: ThreadLocalResource>(&self) -> bool {
        self.info
            .get_resource_id(TypeId::of::<R>())
            .and_then(|id| self.storage.get(id)?.get_ticks())
            .map(|ticks| ticks.is_changed(self.last_tick, self.curr_tick))
            .unwrap_or(false)
    }

    /// Returns a reference to the resource.
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist. Use [`get_resource`](ThreadLocals::get_resource)
    /// instead if you want to handle this case.
    ///
    /// If you want to instead insert a value if the resource does not exist, use
    /// [`get_resource_or_insert_with`](ThreadLocals::get_resource_or_insert_with).
    #[inline]
    #[track_caller]
    pub fn resource<R: ThreadLocalResource>(&self) -> &R {
        match self.get_resource() {
            Some(x) => x,
            None => panic!(
                "Requested resource {} does not exist. Did you insert it?",
                std::any::type_name::<R>()
            ),
        }
    }

    /// Returns a mutable reference to the resource.
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist. Use [`get_resource_mut`](ThreadLocals::get_resource_mut)
    /// instead if you want to handle this case.
    ///
    /// If you want to instead insert a value if the resource does not exist, use
    /// [`get_resource_or_insert_with`](ThreadLocals::get_resource_or_insert_with).
    #[inline]
    #[track_caller]
    pub fn resource_mut<R: ThreadLocalResource>(&mut self) -> Mut<'_, R> {
        match self.get_resource_mut() {
            Some(x) => x,
            None => panic!(
                "Requested resource {} does not exist. Did you insert it?",
                std::any::type_name::<R>()
            ),
        }
    }

    /// Returns a reference to the resource, if it exists.
    #[inline]
    pub fn get_resource<R: ThreadLocalResource>(&self) -> Option<&R> {
        let id = self.info.get_resource_id(TypeId::of::<R>())?;
        // SAFETY: `id` was derived from `R` directly
        unsafe { self.get_resource_by_id(id).map(|ptr| ptr.deref()) }
    }

    /// Returns a mutable reference to the resource, if it exists.
    #[inline]
    pub fn get_resource_mut<R: ThreadLocalResource>(&mut self) -> Option<Mut<'_, R>> {
        // SAFETY: exclusive access is enforced
        unsafe { self.get_resource_unchecked_mut() }
    }

    /// Returns a mutable reference to the resource. If the resource does not exist, calls `f` and
    /// inserts its result first.
    #[inline]
    pub fn get_resource_or_insert_with<R: ThreadLocalResource>(
        &mut self,
        f: impl FnOnce() -> R,
    ) -> Mut<'_, R> {
        if !self.contains_resource::<R>() {
            self.insert_resource(f());
        }
        self.resource_mut()
    }

    /// Returns mutable reference to the resource of the given type, if it exists.
    ///
    /// # Safety
    ///
    /// The caller must ensure that this reference is unique.
    #[inline]
    pub unsafe fn get_resource_unchecked_mut<R: ThreadLocalResource>(&self) -> Option<Mut<'_, R>> {
        let id = self.info.get_resource_id(TypeId::of::<R>())?;
        // SAFETY: `id` was derived from `R` directly
        unsafe {
            self.get_resource_mut_by_id(id)
                .map(|ptr| ptr.with_type::<R>())
        }
    }

    /// # Safety
    ///
    /// The caller must ensure that `id` is assigned to type `R`.
    #[inline]
    pub(crate) unsafe fn get_resource_by_id(&self, id: ComponentId) -> Option<Ptr> {
        self.storage.get(id)?.get_data()
    }

    /// # Safety
    ///
    /// The caller must ensure that `id` is assigned to type `R` and that this reference is unique.
    #[inline]
    pub(crate) unsafe fn get_resource_mut_by_id(&self, id: ComponentId) -> Option<MutUntyped<'_>> {
        // SAFETY: caller ensures unaliased access
        let (ptr, ticks) = unsafe { self.get_resource_with_ticks(id)? };

        // SAFETY: caller ensures unaliased access
        let ticks = unsafe { TicksMut::from_tick_cells(ticks, self.last_tick, self.curr_tick) };

        Some(MutUntyped {
            // SAFETY: caller ensures unaliased access
            value: unsafe { ptr.assert_unique() },
            ticks,
        })
    }

    /// Returns untyped references to the data and change ticks of a resource.
    ///
    /// # Safety
    ///
    /// The caller must ensure that mutable references have no aliases.
    #[inline]
    pub(crate) unsafe fn get_resource_with_ticks(
        &self,
        id: ComponentId,
    ) -> Option<(Ptr<'_>, TickCells)> {
        self.storage.get(id)?.get_with_ticks()
    }

    /// Inserts a new resource with the given `value`. Will replace the value if it already existed.
    ///
    /// **Prefer the typed API [`ThreadLocals::insert_resource`] when possible. Only use this if
    /// the actual types are not known at compile time.**
    ///
    /// # Safety
    /// - `id` must already exist in [`ThreadLocals`]
    /// - `value` must be a valid value of the type represented by `id`
    #[inline]
    pub(crate) unsafe fn insert_resource_by_id(&mut self, id: ComponentId, value: OwningPtr<'_>) {
        let curr_tick = self.curr_tick;
        // SAFETY: caller ensures the value is a valid value of the type given by `id`
        unsafe {
            self.initialize_resource_internal(id)
                .insert(value, curr_tick);
        }
    }

    /// # Safety
    /// `id` must be valid for this world
    #[inline]
    unsafe fn initialize_resource_internal(&mut self, id: ComponentId) -> &mut ResourceData<false> {
        self.storage
            .initialize_with(id, &self.info, || ArchetypeComponentId::new(id.index()))
    }

    /// Temporarily removes `R` from the [`ThreadLocals`], then re-inserts it before returning.
    pub fn resource_scope<R: ThreadLocalResource, T>(
        &mut self,
        f: impl FnOnce(&mut ThreadLocals, Mut<R>) -> T,
    ) -> T {
        let id = self
            .info
            .get_resource_id(TypeId::of::<R>())
            .unwrap_or_else(|| panic!("resource does not exist: {}", std::any::type_name::<R>()));

        let (ptr, mut ticks) = self
            .storage
            .get_mut(id)
            .and_then(|info| info.remove())
            .unwrap_or_else(|| panic!("resource does not exist: {}", std::any::type_name::<R>()));

        // Read the value onto the stack to avoid potential &mut aliasing.
        // SAFETY: pointer is of type R
        let mut value = unsafe { ptr.read::<R>() };
        let value_mut = Mut {
            value: &mut value,
            ticks: TicksMut {
                added: &mut ticks.added,
                changed: &mut ticks.changed,
                last_run: self.last_tick,
                this_run: self.curr_tick,
            },
        };

        let result = f(self, value_mut);
        assert!(
            !self.contains_resource::<R>(),
            "Resource `{}` was inserted during a call to `resource_scope`.\n\
            This is not allowed as the original resource is re-inserted after `f` is invoked.",
            std::any::type_name::<R>()
        );

        OwningPtr::make(value, |ptr| {
            // SAFETY: pointer is of type R
            unsafe {
                self.storage
                    .get_mut(id)
                    .map(|info| info.insert_with_ticks(ptr, ticks))
                    .unwrap();
            }
        });

        result
    }

    pub(crate) fn update_change_tick(&mut self) {
        let tick = self.curr_tick.get();
        self.curr_tick.set(tick.wrapping_add(1));
    }
}

/// Type alias for tasks that access thread-local data.
pub type ThreadLocalTask = Box<dyn FnOnce() + Send + 'static>;

/// An error returned from the [`ThreadLocalTaskSender::send_task`] function.
///
/// A send operation can only fail if the receiving end of a channel is disconnected,
/// implying that the data could never be received. The error contains the data that
/// was sent as a payload so it can be recovered.
#[derive(Debug)]
pub struct ThreadLocalTaskSendError<T>(pub T);

/// Channel for sending [`ThreadLocalTask`] instances.
pub trait ThreadLocalTaskSender: Send + 'static {
    /// Attempts to send a task over this channel, returning it back if it could not be sent.
    fn send_task(
        &mut self,
        task: ThreadLocalTask,
    ) -> Result<(), ThreadLocalTaskSendError<ThreadLocalTask>>;
}

/// A [`Resource`] that enables the use of the [`ThreadLocal`] system parameter.
#[derive(Resource)]
struct ThreadLocalChannel {
    thread: std::thread::ThreadId,
    sender: Box<dyn ThreadLocalTaskSender>,
}

// SAFETY: The pointer to the thread-local storage is only dereferenced in its owning thread.
unsafe impl Send for ThreadLocalChannel {}

// SAFETY: The pointer to the thread-local storage is only dereferenced in its owning thread.
// Likewise, all operations require an exclusive reference, so there can be no races.
unsafe impl Sync for ThreadLocalChannel {}

/// A guard to access [`ThreadLocals`].
pub struct ThreadLocalStorage {
    thread: std::thread::ThreadId,
    // !Send + !Sync
    _marker: PhantomData<*const ()>,
}

impl Default for ThreadLocalStorage {
    fn default() -> Self {
        Self {
            thread: std::thread::current().id(),
            _marker: PhantomData,
        }
    }
}

impl ThreadLocalStorage {
    /// Constructs a new instance of [`ThreadLocalStorage`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes all resources.
    pub fn clear(&mut self) {
        // Drop resources normally to avoid the caveats described in
        // https://doc.rust-lang.org/std/thread/struct.LocalKey.html
        TLS.replace(ThreadLocals::new());
    }

    /// Inserts a new resource with its default value.
    ///
    /// If the resource already exists, nothing happens.
    #[inline]
    pub fn init_resource<R: ThreadLocalResource + Default>(&mut self) {
        TLS.with_borrow_mut(|tls| {
            tls.init_resource::<R>();
        });
    }

    /// Inserts a new resource with the given `value`.
    ///
    /// Resources are "unique" data of a given type. If you insert a resource of a type that already
    /// exists, you will overwrite any existing data.
    #[inline]
    pub fn insert_resource<R: ThreadLocalResource>(&mut self, value: R) {
        TLS.with_borrow_mut(|tls| {
            tls.insert_resource(value);
        });
    }

    /// Removes the resource of a given type and returns it, if it exists.
    #[inline]
    pub fn remove_resource<R: ThreadLocalResource>(&mut self) -> Option<R> {
        TLS.with_borrow_mut(|tls| tls.remove_resource::<R>())
    }

    /// Temporarily removes `R` from the [`ThreadLocals`], then re-inserts it before returning.
    pub fn resource_scope<R: ThreadLocalResource, T>(
        &mut self,
        f: impl FnOnce(&mut ThreadLocals, Mut<R>) -> T,
    ) -> T {
        TLS.with_borrow_mut(|tls| tls.resource_scope(f))
    }

    /// Inserts a channel into `world` that systems in `world` (via [`ThreadLocal`]) can use to
    /// access the underlying [`ThreadLocals`].
    pub fn insert_channel<S>(&self, world: &mut World, sender: S)
    where
        S: ThreadLocalTaskSender,
    {
        let channel = ThreadLocalChannel {
            thread: self.thread,
            sender: Box::new(sender),
        };

        world.insert_resource(channel);
    }

    /// Removes the channel previously added by [`insert_channel`](ThreadLocalStorage::insert_channel)
    /// from `world`, if it exists.
    pub fn remove_channel(&self, world: &mut World) {
        world.remove_resource::<ThreadLocalChannel>();
    }
}

enum ThreadLocalAccess<'a> {
    Direct,
    Indirect(&'a mut dyn ThreadLocalTaskSender),
}

#[doc(hidden)]
pub struct ThreadLocalState {
    component_id: ComponentId,
    last_run: Tick,
}

/// A [`SystemParam`] that grants scoped access to the thread-local data of the main thread.
pub struct ThreadLocal<'w, 's> {
    access: ThreadLocalAccess<'w>,
    last_run: &'s mut Tick,
}

impl ThreadLocal<'_, '_> {
    /// Runs `f` in a scope that has access to the thread-local resources.
    pub fn run<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut ThreadLocals) -> T + Send,
        T: Send + 'static,
    {
        match self.access {
            ThreadLocalAccess::Direct => self.run_direct(f),
            ThreadLocalAccess::Indirect(_) => self.run_indirect(f),
        }
    }

    fn run_direct<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut ThreadLocals) -> T + Send,
        T: Send + 'static,
    {
        debug_assert!(matches!(self.access, ThreadLocalAccess::Direct));

        TLS.with_borrow_mut(|tls| {
            tls.update_change_tick();
            let saved = std::mem::replace(&mut tls.last_tick, *self.last_run);
            let result = f(tls);
            tls.last_tick = saved;
            *self.last_run = tls.curr_tick;
            result
        })
    }

    fn run_indirect<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut ThreadLocals) -> T + Send,
        T: Send + 'static,
    {
        let ThreadLocalAccess::Indirect(ref mut sender) = self.access else {
            unreachable!()
        };

        let system_tick = *self.last_run;
        let (result_tx, result_rx) = std::sync::mpsc::sync_channel(1);
        let task = move || {
            TLS.with_borrow_mut(|tls| {
                tls.update_change_tick();
                let saved = std::mem::replace(&mut tls.last_tick, system_tick);
                // we want to propagate to caller instead of panicking in the main thread
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    (f(tls), tls.curr_tick)
                }));
                tls.last_tick = saved;
                result_tx.send(result).unwrap();
            });
        };

        let task: Box<dyn FnOnce() + Send> = Box::new(task);
        // SAFETY: This function will block the calling thread until `f` completes,
        // so any captured references in `f` will remain valid until then.
        let task: Box<dyn FnOnce() + Send + 'static> = unsafe { std::mem::transmute(task) };

        // Send task to the main thread.
        sender
            .send_task(task)
            .unwrap_or_else(|_| panic!("receiver missing"));

        // Wait to receive result back from the main thread.
        match result_rx.recv().unwrap() {
            Ok((result, tls_tick)) => {
                *self.last_run = tls_tick;
                result
            }
            Err(payload) => {
                std::panic::resume_unwind(payload);
            }
        }
    }
}

// SAFETY: This system param does not borrow any data from the world.
unsafe impl SystemParam for ThreadLocal<'_, '_> {
    type State = ThreadLocalState;
    type Item<'w, 's> = ThreadLocal<'w, 's>;

    fn init_state(
        world: &mut crate::prelude::World,
        system_meta: &mut crate::system::SystemMeta,
    ) -> Self::State {
        // `ThreadLocalTaskSender` can't require `Sync` because `winit`'s `EventLoopProxy` is not
        // `Sync`, so we need exclusive access to `ThreadLocalChannel` here.
        let component_id =
            crate::system::ResMut::<ThreadLocalChannel>::init_state(world, system_meta);

        ThreadLocalState {
            component_id,
            last_run: Tick::new(0),
        }
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &crate::system::SystemMeta,
        world: UnsafeWorldCell<'world>,
        curr_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        let accessor = crate::system::ResMut::<ThreadLocalChannel>::get_param(
            &mut state.component_id,
            system_meta,
            world,
            curr_tick,
        );

        let access = if std::thread::current().id() == accessor.thread {
            ThreadLocalAccess::Direct
        } else {
            ThreadLocalAccess::Indirect(&mut *accessor.into_inner().sender)
        };

        ThreadLocal {
            access,
            last_run: &mut state.last_run,
        }
    }
}
