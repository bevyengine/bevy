use downcast_rs::{impl_downcast, Downcast};

use crate::{App, InternedAppLabel};
use std::any::Any;

/// Plugin state in the application
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginState {
    /// Plugin is not initialized.
    #[default]
    Idle,
    /// Plugin is initialized.
    Init,
    /// Plugin is being built.
    SettingUp,
    /// Plugin is being configured.
    Configuring,
    /// Plugin configuration is finishing.
    Finalizing,
    /// Plugin configuration is completed.
    Done,
    /// Plugin resources are cleaned up.
    Cleaned,
}

impl PluginState {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Idle => Self::Init,
            Self::Init => Self::SettingUp,
            Self::SettingUp => Self::Configuring,
            Self::Configuring => Self::Finalizing,
            Self::Finalizing => Self::Done,
            s => unreachable!("Cannot handle {:?} state", s),
        }
    }
}

/// A collection of Bevy app logic and configuration.
///
/// Plugins configure an [`App`]. When an [`App`] registers a plugin,
/// the plugin's [`Plugin::init`] function is run. By default, a plugin
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
/// * the app calls [`Plugin::init`] immediately, and register the plugin
/// * once the app started, it will wait for all registered [`Plugin::ready`] to return `true`
/// * it will then call all registered [`Plugin::finalize`]
/// * and call all registered [`Plugin::cleanup`]
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
///     fn init(&self, app: &mut App) {
///         if self.flicker_damping {
///             app.add_systems(PostUpdate, damp_flickering);
///         }
///     }
/// }
/// # fn damp_flickering() {}
/// ````
pub trait Plugin: Downcast + Any + Send + Sync {
    /// Returns required sub apps before finalizing this plugin.
    fn required_sub_apps(&self) -> Vec<InternedAppLabel> {
        Vec::new()
    }

    /// Pre-configures the [`App`] to which this plugin is added. This is by convention executed before
    /// the event loop is started.
    fn init(&self, _app: &mut App) {
        // do nothing
    }

    /// Deprecated, see [`Plugin::init`]
    #[deprecated = "Use `Plugin::init` instead"]
    fn build(&self, app: &mut App) {
        self.init(app);
    }

    /// Is the plugin ready to be set up?
    fn ready_to_setup(&self, _app: &mut App) -> bool {
        true
    }

    /// Sets the [`Plugin`] resources and systems. This is by convention executed inside an event loop.
    fn setup(&self, _app: &mut App) {
        // do nothing
    }

    /// Is the plugin ready to be configured?
    fn ready_to_configure(&self, _app: &mut App) -> bool {
        true
    }

    /// Configures the [`App`] to which this plugin is added. This can
    /// be useful for plugins that needs completing asynchronous configuration.
    fn configure(&self, _app: &mut App) {
        // do nothing
    }

    /// Is the plugin ready to be finalized?
    fn ready_to_finalize(&self, _app: &mut App) -> bool {
        true
    }

    /// Finalizes this plugin to the [`App`]. This can
    /// be useful for plugins that depends on another plugin asynchronous setup, like the renderer.
    fn finalize(&self, _app: &mut App) {
        // do nothing
    }

    /// Deprecated, see [`Plugin::finalize`]
    #[deprecated = "Use `Plugin::finalize` instead"]
    fn finish(&self, app: &mut App) {
        self.finalize(app);
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
        std::any::type_name::<Self>()
    }

    /// If the plugin can be meaningfully instantiated several times in an [`App`],
    /// override this method to return `false`.
    fn is_unique(&self) -> bool {
        true
    }

    /// Checks all required [`SubApp`]s.
    fn check_required_sub_apps(&self, app: &App) -> bool {
        self.required_sub_apps()
            .iter()
            .all(|s| app.contains_sub_app(*s))
    }
}

impl_downcast!(Plugin);

impl<T: Fn(&mut App) + Send + Sync + 'static> Plugin for T {}

/// A type representing an unsafe function that returns a mutable pointer to a [`Plugin`].
/// It is used for dynamically loading plugins.
///
/// See `bevy_dynamic_plugin/src/loader.rs#dynamically_load_plugin`.
#[deprecated(
    since = "0.14.0",
    note = "The current dynamic plugin system is unsound and will be removed in 0.15."
)]
pub type CreatePlugin = unsafe fn() -> *mut dyn Plugin;

/// Types that represent a set of [`Plugin`]s.
///
/// This is implemented for all types which implement [`Plugin`],
/// [`PluginGroup`](super::PluginGroup), and tuples over [`Plugins`].
pub trait Plugins<Marker>: sealed::Plugins<Marker> {}

impl<Marker, T> Plugins<Marker> for T where T: sealed::Plugins<Marker> {}

mod sealed {
    use bevy_utils::all_tuples;

    use crate::{App, AppError, Plugin, PluginGroup};

    pub trait Plugins<Marker> {
        fn add_to_app(self, app: &mut App);
    }

    pub struct PluginMarker;
    pub struct PluginGroupMarker;
    pub struct PluginsTupleMarker;

    impl<P: Plugin> Plugins<PluginMarker> for P {
        #[track_caller]
        fn add_to_app(self, app: &mut App) {
            if let Err(AppError::DuplicatePlugin { plugin_name }) =
                app.add_boxed_plugin(Box::new(self))
            {
                panic!("Error adding plugin {plugin_name}: plugin was already added in application")
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
        ($(($param: ident, $plugins: ident)),*) => {
            impl<$($param, $plugins),*> Plugins<(PluginsTupleMarker, $($param,)*)> for ($($plugins,)*)
            where
                $($plugins: Plugins<$param>),*
            {
                #[allow(non_snake_case, unused_variables)]
                #[track_caller]
                fn add_to_app(self, app: &mut App) {
                    let ($($plugins,)*) = self;
                    $($plugins.add_to_app(app);)*
                }
            }
        }
    }

    all_tuples!(impl_plugins_tuples, 0, 15, P, S);
}
