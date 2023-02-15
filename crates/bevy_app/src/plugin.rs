use downcast_rs::{impl_downcast, Downcast};

use crate::App;
use std::any::Any;

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
pub trait Plugin: Downcast + Any + Send + Sync {
    /// Configures the [`App`] to which this plugin is added.
    fn build(&self, app: &mut App);

    /// Runs after all plugins are built, but before the app runner is called.
    /// This can be useful if you have some resource that other plugins need during their build step,
    /// but after build you want to remove it and send it to another thread.
    fn setup(&self, _app: &mut App) {
        // do nothing
    }

    /// Configures a name for the [`Plugin`] which is primarily used for checking plugin
    /// uniqueness and debugging.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// If the plugin can be meaningfully instantiated several times in an [`App`](crate::App),
    /// override this method to return `false`.
    fn is_unique(&self) -> bool {
        true
    }
}

impl_downcast!(Plugin);

/// A type representing an unsafe function that returns a mutable pointer to a [`Plugin`].
/// It is used for dynamically loading plugins.
///
/// See `bevy_dynamic_plugin/src/loader.rs#dynamically_load_plugin`.
pub type CreatePlugin = unsafe fn() -> *mut dyn Plugin;

pub(super) mod sealed {
    use bevy_ecs::all_tuples;

    use crate::{App, Plugin, PluginGroup, PluginGroupBuilder};

    pub trait IntoPlugin<Params> {
        type Plugin: Plugin;
        fn into_plugin(self, app: &mut App) -> Self::Plugin;
    }

    pub trait IntoPluginGroup<Params>: IntoPluginGroupBuilder<Params> {}

    pub trait IntoPluginGroupBuilder<Params> {
        fn into_plugin_group_builder(self, app: &mut App) -> PluginGroupBuilder;
    }

    pub struct IsPlugin;
    pub struct IsPluginGroup;
    pub struct IsFunction;

    impl<P: Plugin> IntoPlugin<IsPlugin> for P {
        type Plugin = Self;
        fn into_plugin(self, _: &mut App) -> Self {
            self
        }
    }

    impl<P: Plugin> IntoPluginGroupBuilder<IsPlugin> for P {
        fn into_plugin_group_builder(self, _: &mut App) -> PluginGroupBuilder {
            PluginGroupBuilder::from_plugin(self)
        }
    }

    impl<P: PluginGroup> IntoPluginGroupBuilder<IsPluginGroup> for P {
        fn into_plugin_group_builder(self, _: &mut App) -> PluginGroupBuilder {
            self.build()
        }
    }

    impl<P: PluginGroup> IntoPluginGroup<IsPluginGroup> for P {}

    impl<F: FnOnce(&mut App) -> P, P: Plugin> IntoPlugin<IsFunction> for F {
        type Plugin = P;

        fn into_plugin(self, app: &mut App) -> Self::Plugin {
            self(app)
        }
    }

    impl<F: FnOnce(&mut App) -> PG, PG: PluginGroup> IntoPluginGroupBuilder<IsFunction> for F {
        fn into_plugin_group_builder(self, app: &mut App) -> PluginGroupBuilder {
            self(app).build()
        }
    }

    impl<F: FnOnce(&mut App) -> PG, PG: PluginGroup> IntoPluginGroup<IsFunction> for F {}

    macro_rules! impl_plugin_collection {
        ($(($param: ident, $plugins: ident)),*) => {
            impl<$($param, $plugins),*> IntoPluginGroupBuilder<($($param,)*)> for ($($plugins,)*)
            where
                $($plugins: IntoPluginGroupBuilder<$param>),*
            {
                #[allow(non_snake_case, unused_variables)]
                fn into_plugin_group_builder(self, app: &mut App) -> PluginGroupBuilder {
                    let ($($plugins,)*) = self;
                    PluginGroupBuilder::merge(vec![$($plugins.into_plugin_group_builder(app),)*])
                }
            }

            impl<$($param, $plugins),*> IntoPluginGroup<($($param,)*)> for ($($plugins,)*)
            where
                $($plugins: IntoPluginGroupBuilder<$param>),*
            {}
        }
    }

    all_tuples!(impl_plugin_collection, 0, 15, P, S);
}
