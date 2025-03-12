use core::{
    cell::{RefCell, RefMut},
    marker::PhantomData,
    pin::Pin,
    task::Poll,
};

use bevy_ecs::{
    resource::Resource,
    world::{Mut, World},
};
use thiserror::Error;

use crate::App;

/// Context provided to an `Async` plugin,
/// giving it access to the app and letting it wait
/// for its dependencies.
pub struct PluginContext<'app> {
    pub(crate) inner: &'app RefCell<PluginContextInner<'app>>,
}

pub(crate) struct PluginContextInner<'a> {
    pub(crate) app: &'a mut App,
    pub(crate) progress: TickProgress,
}

/// Whether a future has made progress.
/// Used to detect a stuck state in plugin loading.
#[derive(PartialEq, Debug)]
pub(crate) enum TickProgress {
    Unknown,
    NoProgress,
    MadeProgress,
    Stuck,
}

impl PluginContext<'_> {
    #[track_caller]
    pub(crate) fn borrow_mut<'ctx>(&'ctx self) -> (RefMut<'ctx, App>, RefMut<'ctx, TickProgress>) {
        let inner = self
            .inner
            .try_borrow_mut()
            .expect("Error during plugin building: Reference to app was held across await point");
        RefMut::map_split(inner, |inner| (inner.app, &mut inner.progress))
    }

    /// Get a mutable reference to the app.
    #[track_caller]
    pub fn app(&mut self) -> RefMut<'_, App> {
        self.borrow_mut().0
    }

    /// Get a mutable reference to the app.
    #[track_caller]
    pub fn world(&mut self) -> RefMut<'_, World> {
        RefMut::map(self.borrow_mut().0, |app| app.world_mut())
    }

    /// Wait for a dependency to become available and return it.
    ///
    /// The function will be rerun until either it returns `Some`,
    /// or no other plugin is making progress either.
    pub fn get<'ctx, F, Return>(
        &'ctx self,
        function: F,
    ) -> impl Future<Output = Result<Return, StuckError>> + 'ctx
    where
        F: Fn(RefMut<'ctx, App>) -> Option<Return> + 'ctx,
        Return: 'ctx,
    {
        struct Wait<'ctx, 'a, F: 'ctx, R> {
            ctx: &'ctx PluginContext<'a>,
            function: F,
            _return: PhantomData<R>,
        }
        impl<'ctx, F, Return> Future for Wait<'ctx, '_, F, Return>
        where
            F: Fn(RefMut<'ctx, App>) -> Option<Return>,
            Return: 'ctx,
        {
            type Output = Result<Return, StuckError>;

            #[track_caller]
            fn poll(self: Pin<&mut Self>, _: &mut core::task::Context<'_>) -> Poll<Self::Output> {
                let (app, mut progress) = self.ctx.borrow_mut();
                if *progress == TickProgress::Stuck {
                    *progress = TickProgress::MadeProgress;
                    Poll::Ready(Err(StuckError))
                } else if let Some(res) = (self.function)(app) {
                    Poll::Ready(Ok(res))
                } else {
                    if *progress == TickProgress::Unknown {
                        *progress = TickProgress::NoProgress;
                    }
                    Poll::Pending
                }
            }
        }
        Wait {
            ctx: self,
            function,
            _return: PhantomData::<Return>,
        }
    }

    /// Wait for a condition to become true.
    ///
    /// The function will be rerun until either it returns `true`,
    /// or no other plugin is making progress either.
    ///
    /// ```rust
    /// # use bevy_ecs::resource::Resource;
    /// # let ctx: bevy_app::PluginContext<'static> = todo!();
    /// # async move {
    /// ctx.wait(|app| {
    ///     app.world()
    ///         .get_resource::<MyLoadState>()
    ///         .map(|state| *state == MyLoadState::Done)
    ///         .unwrap_or_default()
    /// })
    /// .await
    /// .unwrap();
    /// # }
    /// # #[derive(Resource, PartialEq, Eq)]
    /// # enum MyLoadState { Done }
    /// ```
    pub fn wait<'ctx, F>(
        &'ctx self,
        function: F,
    ) -> impl Future<Output = Result<(), StuckError>> + 'ctx
    where
        F: Fn(RefMut<'ctx, App>) -> bool + 'ctx,
    {
        self.get(move |app| function(app).then_some(()))
    }

    /// Wait for a resource to become available and return a mutable reference to it.
    ///
    /// Holding this reference across sync points will panic.
    pub fn resource<'ctx, R: Resource>(
        &'ctx self,
    ) -> impl Future<Output = Result<RefMut<'ctx, R>, StuckError>> + 'ctx {
        self.get(|app| {
            RefMut::filter_map(app, |app| {
                app.world_mut().get_resource_mut::<R>().map(Mut::into_inner)
            })
            .ok()
        })
    }

    /// Wait for a plugin to be added to the app.
    /// For most plugins, their name is their type name.
    pub fn plugin_added<'ctx>(
        &'ctx self,
        name: &'ctx str,
    ) -> impl Future<Output = Result<(), StuckError>> + 'ctx {
        self.wait(|app| app.main().added_plugins.contains(name))
    }

    /// Wait for a plugin's [`build_async`] to finish running.
    /// For most plugins, their name is their type name.
    ///
    /// When writing a library, try to wait for the specific data or conditions
    /// you need instead if possible to improve modularity.
    ///
    /// [`build_async`]: crate::Plugin::build_async
    pub fn plugin_completed<'ctx>(
        &'ctx self,
        name: &'ctx str,
    ) -> impl Future<Output = Result<(), StuckError>> + 'ctx {
        self.wait(|app| app.main().completed_plugins.contains(name))
    }
}

/// Returned when no plugin is making progress.
#[derive(Debug, Error)]
#[error("No plugin is making progress")]
pub struct StuckError;
