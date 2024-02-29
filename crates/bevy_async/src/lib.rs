//! `async` system (coroutine) support for the Bevy ECS.
//!
//! Coroutine systems allow you to use `async.await` to write systems which run
//! over multiple frames. However, in order to satisfy safe ECS access patterns,
//! the full benefit of `async` cannot be taken advantage of. Specifically, any
//! system parameters which are accessed cannot be held over an `await` point,
//! as this would block other systems from accessing them.
//!
//! Instead, access to system parameters is limited to within synchronous
//! sections of the larger coroutine, accessed by a closure-taking `with` API
//! similar to how thread locals or scoped threads function.
//!
//! Additionally, note that coroutine systems don't participate in the normal
//! wake-up system that other futures do. Instead, a coroutine system will be
//! polled *once every frame* (whenever the system's run criteria is met). As
//! such, it is highly advised to spawn any actual async work on bevy's task
//! pools ([`IoTaskPool`], [`ComputeTaskPool`], [`AsyncComputeTaskPool`]) and
//! `.await`ing the [`Task`] handle they provide instead. This will ensure that
//! the futures are woken and driven to completion in a normal fashion.
//!
//! [`IoTaskPool`]: bevy_tasks::IoTaskPool
//! [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
//! [`AsyncComputeTaskPool`]: bevy_tasks::AsyncComputeTaskPool
//! [`Task`]: bevy_tasks::Task
//!
//! The final major restriction is that coroutine systems must loop infinitely.
//! In this way, coroutine systems are essentially a library-level pollyfill
//! using [MCP-49 style `FnPin`/`yield`][fnpin] to write resumable systems
//! made implementable by reusing the ECS system parameter infrastructure.
//!
//! [fnpin]: https://lang-team.rust-lang.org/design_notes/general_coroutines.html
//!
//! The type of a coroutine system is `CoroutineSystem<Params, FnItem>`. Both
//! because this is an unweildy unnamable type and because it requires runtime
//! (non-`const`) construction, the standard way to define a coroutine system is
//! a function returning <code>impl [System]</code>. Once it becomes possible to
//! use [opaque types in type aliases][tait], a `#[coroutine]` attribute macro
//! can be provided to unify the user syntax by defining an [`IntoSystem`] item.
//!
//! [tait]: https://github.com/rust-lang/rust/issues/63063
//!
//! # Example
//!
//! Coroutine systems are defined using the [`co!`] macro wrapping an `async`
//! closure.
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! # use bevy_tasks::prelude::*;
//! # use bevy_async::*;
//! fn make_system() -> impl System {
//!     // Declare system parameters as with standard function systems.
//!     co!(async move |state: Local<usize>| loop {
//!         let name = String::from("bevy_async");
//!         // Access the parameters by using co_with!
//!         let _ = co_with!(|state| {
//!             // Within this closure, the parameters are available.
//!             println!("The local's current value is {}", *state);
//!             // The closure borrows normally from the containing scope.
//!             println!("The name is {}", name);
//!             // The closure can pass state back to the async context.
//!             (&*name, *state)
//!         });
//!         // Outside co_with! is an async context where you can use await.
//!         // It's best practice to spawn async work onto a task pool
//!         // and await the task handle to minimize redundant polling.
//!         AsyncComputeTaskPool::get().spawn(async move {
//!             // ...
//! #           std::thread::sleep(std::time::Duration::from_secs(1));
//!         }).await;
//!     })
//! }
//! # bevy_ecs::system::assert_is_system(make_system());
//! ```
//!
//! It's not possible to access system parameters outside of `co_with!`
//!
//! ```compile_fail
//! # use bevy_ecs::prelude::*;
//! # use bevy_async::*;
//! fn make_system() -> impl System {
//!    co!(async move |state: Local<usize>| loop {
//!       &*state; //~ error[E0381]: used binding `state` isn't initialized
//!    })
//! }
//! ```
//!
//! nor is it possible to smuggle them out of the closure.
//!
//! ```compile_fail
//! # use bevy_ecs::prelude::*;
//! # use bevy_async::*;
//! fn make_system() -> impl System {
//!     co!(async move |state: Local<usize>| loop {
//!         let state = co_with!(|state| state);
//!         //^ error: lifetime may not live long enough
//!     })
//! }
//! ```
//!
//! ```compile_fail
//! # use bevy_ecs::prelude::*;
//! # use bevy_async::*;
//! fn make_system() -> impl System {
//!     co!(async move |state: Local<usize>| loop {
//!         let mut smuggle = None;
//!         co_with!(|state| {
//!             smuggle = Some(state);
//!             //^ error[E0521]: borrowed data escapes outside of closure
//!         });
//!     })
//! }
//! ```

use bevy_ecs::{
    prelude::*,
    system::{SystemParam, SystemParamItem, SystemState},
};
use futures_lite::future::{block_on, poll_once, Future};
use pin_project_lite::pin_project;
use std::{
    convert::Infallible as Never, marker::PhantomPinned, mem::MaybeUninit, pin::Pin,
    ptr::addr_of_mut,
};

// # SAFETY
//
// This crate implements a very simplistic channel and uses it to smuggle the
// world reference and system state in as pseudo-resume-arguments to an `async`
// closure by closing over a pinned memory location used to pass them in.
//
// In order for this scheme to be sound, several things must go right:
//
// - The "channel" must be pinned before constructing the future. This is
//   achieved via manual piecewise initialization in CoroutineSystem::new.
// - The resume arguments must always be newly populated before polling the
//   future. This is handled by the write in <CoroutineSystem as System>::run.
// - The resume arguments must not be used over an await point. The co! macro
//   ensures this by providing access to the resume arguments only by a sync
//   callback API utilizing lifetime elision in function signatures to ensure
//   the lifetimes are sufficiently shortened, as with typical function systems.
//
// That all this works out without requiring the API consumer to write any more
// lifetime anotations than they would for a function system is a minor miracle.

/// Message shown when a system isn't initialised
const PARAM_MESSAGE: &str = "Async system's param_state was not found. \
    Did you forget to initialize this system before running it?";

/// Not public API. Used by [`co!`].
#[doc(hidden)]
pub mod __ {
    use super::*;
    pub use futures_lite::future::yield_now;

    pub struct Fetch<Param: SystemParam + 'static>(
        pub(super) Yolo<*mut CoroutineSystemFetchInjection<Param>>,
    );
    impl<Param: SystemParam + 'static> Fetch<Param> {
        /// Fetch system parameters from the stashed world and system state.
        ///
        /// # Safety
        ///
        /// The referenced stash must must contain valid state to safely call
        /// [`SystemState::get_unchecked_manual`] with this `&self` lifetime.
        pub unsafe fn fetch_params_unchecked(&mut self) -> SystemParamItem<Param> {
            let this = &mut *self.0.yolo();
            let system_state = this.system.as_mut().expect(PARAM_MESSAGE);
            let world = &*this.world.yolo();
            system_state.get_unchecked_manual(world)
        }
    }
}
use __::Fetch;

/// Remove the `Send`, `Sync`, and initializedness safety from a type.
struct Yolo<T>(MaybeUninit<T>);
unsafe impl<T> Send for Yolo<T> {}
unsafe impl<T> Sync for Yolo<T> {}
impl<T: Copy> Yolo<T> {
    /// Access the inner value.
    ///
    /// # Safety
    ///
    /// The value must have most recently been written to on this thread.
    unsafe fn yolo(&self) -> T {
        self.0.assume_init()
    }
}

pin_project! {
    struct CoroutineSystemFetchInjection<Param: 'static>
    where
        Param: SystemParam,
    {
        world: Yolo<*const World>,
        system: Option<SystemState<Param>>,
        #[pin]
        // SAFETY: Self must be !Unpin to communicate that this state
        //         is aliased, despite the state itself being Unpin.
        pinned: PhantomPinned,
    }
}

pin_project! {
    struct PinnedCoroutineSystem<Param: 'static, F>
    where
        Param: SystemParam,
    {
        #[pin]
        func: F,
        #[pin]
        state: CoroutineSystemFetchInjection<Param>,
    }
}

/// A coroutine system.
///
/// See the [crate-level documentation](crate) for more information.
pub struct CoroutineSystem<Param, F>
where
    Param: SystemParam + 'static,
{
    pinned: Pin<Box<PinnedCoroutineSystem<Param, F>>>,
}

impl<Param, F> CoroutineSystem<Param, F>
where
    Param: SystemParam + 'static,
{
    fn state_mut(&mut self) -> &mut SystemState<Param> {
        let pinned = self.pinned.as_mut().project();
        let state = pinned.state.project();
        state.system.as_mut().expect(PARAM_MESSAGE)
    }

    fn state(&self) -> &SystemState<Param> {
        let pinned = self.pinned.as_ref().project_ref();
        let state = pinned.state.project_ref();
        state.system.as_ref().expect(PARAM_MESSAGE)
    }

    /// Not public API. Used by [`co!`].
    #[doc(hidden)]
    pub fn new(func: impl FnOnce(Fetch<Param>) -> F) -> Self
    where
        F: Future<Output = Never>,
    {
        let mut pinned = Box::new(MaybeUninit::<PinnedCoroutineSystem<Param, F>>::uninit());
        let pinned = unsafe {
            let pinned_ptr = pinned.as_mut_ptr();
            // SAFETY: pinned_ptr is valid; no references are created.
            let state_ptr = addr_of_mut!((*pinned_ptr).state);
            // SAFETY: pinned_ptr is valid; no references are created.
            let func_ptr = addr_of_mut!((*pinned_ptr).func);
            // SAFETY: state_ptr is valid; (*pinned).state is not yet initialized.
            state_ptr.write(CoroutineSystemFetchInjection {
                world: Yolo(MaybeUninit::uninit()),
                system: None,
                pinned: PhantomPinned,
            });
            // SAFETY: func_ptr is valid; (*pinned).func is not yet initialized.
            // NOTE: func won't unwind; it's just `|_| async { ... }` from co!.
            func_ptr.write(func(Fetch(Yolo(MaybeUninit::new(state_ptr)))));
            // Assert that no uninitialized fields remain.
            let PinnedCoroutineSystem { state: _, func: _ }: PinnedCoroutineSystem<Param, F>;
            // SAFETY: this is Box::assume_init; *pinned was fully initialized above.
            Box::from_raw(Box::into_raw(pinned).cast())
        };
        Self {
            pinned: Box::into_pin(pinned),
        }
    }
}

impl<Param, F> System for CoroutineSystem<Param, F>
where
    Param: SystemParam + 'static,
    F: Send + Sync + 'static,
    F: Future<Output = Never>,
{
    type In = ();
    type Out = ();

    fn name(&self) -> std::borrow::Cow<'static, str> {
        self.state().name()
    }

    fn component_access(&self) -> &bevy_ecs::query::Access<bevy_ecs::component::ComponentId> {
        self.state().component_access()
    }

    fn archetype_component_access(
        &self,
    ) -> &bevy_ecs::query::Access<bevy_ecs::archetype::ArchetypeComponentId> {
        self.state().archetype_component_access()
    }

    fn is_send(&self) -> bool {
        self.state().is_send()
    }

    fn is_exclusive(&self) -> bool {
        false
    }

    unsafe fn run_unsafe(&mut self, _input: (), world: &World) {
        let this = self.pinned.as_mut().project();
        this.state.project().world.0.write(world);
        // FUTURE: this is the standard hack to poll a future once, but it would
        // be more accurate to either provide a "null waker" since we ignore the
        // wakeup, or to provide our own waker that allows us to bridge the wake
        // to the system's run criteria and only poll once the future is awoken.
        // SAFETY:
        // - self.pinned.state has no active interior borrows
        // - self.pinned.state is valid for get_unchecked_manual within this fn
        //   given we use the world ref stashed just above (ensured by caller)
        if let Some(never) = block_on(poll_once(this.func)) {
            match never {}
        }
    }

    fn apply_buffers(&mut self, world: &mut World) {
        self.state_mut().apply(world);
    }

    fn initialize(&mut self, world: &mut World) {
        let this = self.pinned.as_mut().project();
        *this.state.project().system = Some(SystemState::new(world));
    }

    fn update_archetype_component_access(&mut self, world: &World) {
        self.state_mut().update_archetype_component_access(world);
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        self.state_mut().check_change_tick(change_tick);
    }

    fn get_last_change_tick(&self) -> u32 {
        self.state().get_last_change_tick()
    }

    fn set_last_change_tick(&mut self, last_change_tick: u32) {
        self.state_mut().set_last_change_tick(last_change_tick)
    }
}

/// Declare a coroutine system.
///
/// Returns a [`CoroutineSystem`] to be used like any other [`System`].
///
/// See the [crate-level documentation](crate) for more information.
#[macro_export]
macro_rules! co {
    (async move |$($arg:ident: $ArgTy:ty),*$(,)?| loop $body:block) => {
        $crate::CoroutineSystem::<($($ArgTy,)*), _>::new(
            move |mut co: $crate::__::Fetch<($($ArgTy,)*)>| async move {
                // Declare but leave uninitialized bindings for the system
                // parameters: this provides a useful error message if the user
                // tries to use them outside of the `co_with!` callback.
                // Space isn't used on the future's stack, even in debug mode.
                $(#[allow(unused_variables)] let $arg: $ArgTy;)*
                fn co_with<R>(
                    co: &mut $crate::__::Fetch<($($ArgTy,)*)>,
                    f: impl FnOnce($($ArgTy,)*) -> R,
                ) -> R {
                    // SAFETY: The fetch space is sufficiently set up by
                    //         CoroutineSystem before polling this future.
                    let ($($arg,)*) = unsafe { co.fetch_params_unchecked() };
                    f($($arg,)*)
                }
                macro_rules! co_with {($with:expr) => {
                    co_with(&mut co, $with)
                }}
                loop {
                    $body;
                    // Insert a yield point on the loop backedge; this makes
                    // no-await systems work identically to sync fn systems.
                    $crate::__::yield_now().await;
                }
            }
        )
    };
}
