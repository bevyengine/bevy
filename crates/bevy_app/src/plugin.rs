use crate::{App, PluginContext};
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use core::{any::Any, pin::Pin};
use downcast_rs::{impl_downcast, Downcast};

/// A collection of Bevy app logic and configuration.
///
/// Plugins configure an [`App`]. When an [`App`] registers a plugin,
/// the plugin's [`Plugin::build`] function is run. By default, a plugin
/// can only be added once to an [`App`].
///
/// If the plugin may need to be added twice or more, the function [`is_unique()`](Self::is_unique)
/// should be overridden to return `false`. Plugins are considered duplicate if they have the same
/// [`name()`](Self::name). The default `name()` implementation returns the type name, which means
/// generic plugins with different type parameters will not be considered duplicates.
///
/// ## Lifecycle of a plugin
///
/// When adding a plugin to an [`App`]:
/// * the app calls [`Plugin::build`] immediately, and register the plugin
/// * once the app started, it will wait for all registered [`Plugin::ready`] to return `true`
/// * it will then call all registered [`Plugin::finish`]
/// * and call all registered [`Plugin::cleanup`]
/// * finally, it will run [`Plugin::build_async`] for all plugins concurrently
///
/// ## Defining a plugin.
///
/// Most plugins are simply functions that add configuration to an [`App`].
///
/// ```
/// # use bevy_app::{App, Update};
/// App::new().add_plugins(my_plugin).run();
///
/// // This function implements `Plugin`, along with every other `fn(&mut App)`.
/// pub fn my_plugin(app: &mut App) {
///     app.add_systems(Update, hello_world);
/// }
/// # fn hello_world() {}
/// ```
///
/// For more advanced use cases, the `Plugin` trait can be implemented manually for a type.
///
/// ```
/// # use bevy_app::*;
/// pub struct AccessibilityPlugin {
///     pub flicker_damping: bool,
///     // ...
/// }
///
/// impl Plugin for AccessibilityPlugin {
///     fn build(&self, app: &mut App) {
///         if self.flicker_damping {
///             app.add_systems(PostUpdate, damp_flickering);
///         }
///     }
/// }
/// # fn damp_flickering() {}
/// ```
///
/// If you depend on data added by another plugin, define `build_async`:
///
/// ```rust
/// # use bevy_app::*;
/// # use bevy_ecs::resource::Resource;
/// # #[derive(Resource)]
/// # struct Settings {
/// #    pub flicker_damping: bool,
/// # }
/// # struct AccessibilityPlugin;
///
/// impl Plugin for AccessibilityPlugin {
///     async fn build_async(self: Box<Self>, mut ctx: PluginContext<'_>) {
///         // Wait until another plugin makes the resource available
///         let settings = ctx.resource::<Settings>().await.unwrap();
///         if settings.flicker_damping {
///             drop(settings);
///             ctx.app().add_systems(PostUpdate, damp_flickering);
///         }
///     }
/// }
/// # fn damp_flickering() {}
/// ```
pub trait Plugin: Any + Send + Sync {
    /// Configures the [`App`] to which this plugin is added.
    ///
    /// This runs concurrently with other plugins (but in a single thread), so you can use the
    /// methods on [`PluginContext`] to wait for data to be made available by other plugins.
    fn build_async<'ctx>(
        self: Box<Self>,
        _ctx: PluginContext<'ctx>,
    ) -> impl Future<Output = ()> + 'ctx {
        async {}
    }

    /// Configures the [`App`] to which this plugin is added.
    fn build(&self, _app: &mut App) {}

    /// Has the plugin finished its setup? This can be useful for plugins that need something
    /// asynchronous to happen before they can finish their setup, like the initialization of a renderer.
    /// Once the plugin is ready, [`finish`](Plugin::finish) should be called.
    fn ready(&self, _app: &App) -> bool {
        true
    }

    /// Finish adding this plugin to the [`App`], once all plugins registered are ready. This can
    /// be useful for plugins that depends on another plugin asynchronous setup, like the renderer.
    fn finish(&self, _app: &mut App) {
        // do nothing
    }

    /// Runs after all plugins are built and finished, but before the app schedule is executed.
    /// This can be useful if you have some resource that other plugins need during their build step,
    /// but after build you want to remove it and send it to another thread.
    fn cleanup(&self, _app: &mut App) {
        // do nothing
    }

    /// Configures a name for the [`Plugin`] which is primarily used for checking plugin
    /// uniqueness and debugging.
    fn name(&self) -> &str {
        core::any::type_name::<Self>()
    }

    /// If the plugin can be meaningfully instantiated several times in an [`App`],
    /// override this method to return `false`.
    fn is_unique(&self) -> bool {
        true
    }
}

impl<T: Fn(&mut App) + Send + Sync + 'static> Plugin for T {
    fn build(&self, app: &mut App) {
        self(app);
    }
}

/// Dyn-compatible version of [`Plugin`].
pub trait ErasedPlugin: Downcast + Send + Sync {
    /// See [`Plugin::build_async`].
    fn build_async<'a>(
        self: Box<Self>,
        ctx: PluginContext<'a>,
    ) -> Pin<Box<dyn Future<Output = String> + 'a>>;

    /// See [`Plugin::build`].
    fn build(&self, app: &mut App);

    /// See [`Plugin::ready`].
    fn ready(&self, app: &App) -> bool;

    /// See [`Plugin::finish`].
    fn finish(&self, app: &mut App);

    /// See [`Plugin::cleanup`].
    fn cleanup(&self, app: &mut App);

    /// See [`Plugin::name`].
    fn name(&self) -> &str;

    /// See [`Plugin::is_unique`].
    fn is_unique(&self) -> bool;
}

impl_downcast!(ErasedPlugin);

impl<P> ErasedPlugin for P
where
    P: Plugin,
{
    fn build_async<'ctx>(
        self: Box<Self>,
        ctx: PluginContext<'ctx>,
    ) -> Pin<Box<dyn Future<Output = String> + 'ctx>> {
        Box::pin(async move {
            let name = <P as Plugin>::name(&*self).to_owned();
            <P as Plugin>::build_async(self, ctx).await;
            name
        })
    }

    fn build(&self, app: &mut App) {
        Plugin::build(self, app);
    }

    fn ready(&self, app: &App) -> bool {
        Plugin::ready(self, app)
    }

    fn finish(&self, app: &mut App) {
        Plugin::finish(self, app);
    }

    fn cleanup(&self, app: &mut App) {
        Plugin::cleanup(self, app);
    }

    fn name(&self) -> &str {
        Plugin::name(self)
    }

    fn is_unique(&self) -> bool {
        Plugin::is_unique(self)
    }
}

/// Plugins state in the application
#[derive(PartialEq, Eq, Debug, Clone, Copy, PartialOrd, Ord)]
pub enum PluginsState {
    /// Plugins are being added.
    Adding,
    /// All plugins already added are ready.
    Ready,
    /// Finish has been executed for all plugins added.
    Finished,
    /// Cleanup is running.
    Cleaning,
    /// Cleanup has been executed for all plugins added.
    Cleaned,
}

/// A dummy plugin that's to temporarily occupy an entry in an app's plugin registry.
pub(crate) struct PlaceholderPlugin;

impl Plugin for PlaceholderPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Types that represent a set of [`Plugin`]s.
///
/// This is implemented for all types which implement [`Plugin`],
/// [`PluginGroup`](super::PluginGroup), and tuples over [`Plugins`].
pub trait Plugins<Marker>: sealed::Plugins<Marker> {}

impl<Marker, T> Plugins<Marker> for T where T: sealed::Plugins<Marker> {}

mod sealed {
    use alloc::boxed::Box;
    use variadics_please::all_tuples;

    use crate::{App, AppError, ErasedPlugin, PluginGroup};

    pub trait Plugins<Marker> {
        fn add_to_app(self, app: &mut App);
    }

    pub struct PluginMarker;
    pub struct PluginGroupMarker;
    pub struct PluginsTupleMarker;

    impl<P: ErasedPlugin> Plugins<PluginMarker> for P {
        #[track_caller]
        fn add_to_app(self, app: &mut App) {
            if let Err(AppError::DuplicatePlugin { plugin_name }) =
                app.add_boxed_plugin(Box::new(self))
            {
                panic!(
                    "Error adding plugin {plugin_name}: : plugin was already added in application"
                )
            }
        }
    }

    impl<P: PluginGroup> Plugins<PluginGroupMarker> for P {
        #[track_caller]
        fn add_to_app(self, app: &mut App) {
            self.build().finish(app);
        }
    }

    macro_rules! impl_plugins_tuples {
        ($(#[$meta:meta])* $(($param: ident, $plugins: ident)),*) => {
            $(#[$meta])*
            impl<$($param, $plugins),*> Plugins<(PluginsTupleMarker, $($param,)*)> for ($($plugins,)*)
            where
                $($plugins: Plugins<$param>),*
            {
                #[expect(
                    clippy::allow_attributes,
                    reason = "This is inside a macro, and as such, may not trigger in all cases."
                )]
                #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
                #[allow(unused_variables, reason = "`app` is unused when implemented for the unit type `()`.")]
                #[track_caller]
                fn add_to_app(self, app: &mut App) {
                    let ($($plugins,)*) = self;
                    $($plugins.add_to_app(app);)*
                }
            }
        }
    }

    all_tuples!(
        #[doc(fake_variadic)]
        impl_plugins_tuples,
        0,
        15,
        P,
        S
    );
}
