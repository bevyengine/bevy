use crate::{App, AppError, Plugin};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use bevy_platform::collections::hash_map::Entry;
use bevy_utils::TypeIdMap;
use core::any::TypeId;
use log::{debug, warn};

/// A macro for generating a well-documented [`PluginGroup`] from a list of [`Plugin`] paths.
///
/// Every plugin must implement the [`Default`] trait.
///
/// # Example
///
/// ```
/// # use bevy_app::*;
/// #
/// # mod velocity {
/// #     use bevy_app::*;
/// #     #[derive(Default)]
/// #     pub struct VelocityPlugin;
/// #     impl Plugin for VelocityPlugin { fn build(&self, _: &mut App) {} }
/// # }
/// #
/// # mod collision {
/// #     pub mod capsule {
/// #         use bevy_app::*;
/// #         #[derive(Default)]
/// #         pub struct CapsuleCollisionPlugin;
/// #         impl Plugin for CapsuleCollisionPlugin { fn build(&self, _: &mut App) {} }
/// #     }
/// # }
/// #
/// # #[derive(Default)]
/// # pub struct TickratePlugin;
/// # impl Plugin for TickratePlugin { fn build(&self, _: &mut App) {} }
/// #
/// # mod features {
/// #   use bevy_app::*;
/// #   #[derive(Default)]
/// #   pub struct ForcePlugin;
/// #   impl Plugin for ForcePlugin { fn build(&self, _: &mut App) {} }
/// # }
/// #
/// # mod web {
/// #   use bevy_app::*;
/// #   #[derive(Default)]
/// #   pub struct WebCompatibilityPlugin;
/// #   impl Plugin for WebCompatibilityPlugin { fn build(&self, _: &mut App) {} }
/// # }
/// #
/// # mod audio {
/// #   use bevy_app::*;
/// #   #[derive(Default)]
/// #   pub struct AudioPlugins;
/// #   impl PluginGroup for AudioPlugins {
/// #     fn build(self) -> PluginGroupBuilder {
/// #       PluginGroupBuilder::start::<Self>()
/// #     }
/// #   }
/// # }
/// #
/// # mod internal {
/// #   use bevy_app::*;
/// #   #[derive(Default)]
/// #   pub struct InternalPlugin;
/// #   impl Plugin for InternalPlugin { fn build(&self, _: &mut App) {} }
/// # }
/// #
/// plugin_group! {
///     /// Doc comments and annotations are supported: they will be added to the generated plugin
///     /// group.
///     #[derive(Debug)]
///     pub struct PhysicsPlugins {
///         // If referencing a plugin within the same module, you must prefix it with a colon `:`.
///         :TickratePlugin,
///         // If referencing a plugin within a different module, there must be three colons `:::`
///         // between the final module and the plugin name.
///         collision::capsule:::CapsuleCollisionPlugin,
///         velocity:::VelocityPlugin,
///         // If you feature-flag a plugin, it will be automatically documented. There can only be
///         // one automatically documented feature flag, and it must be first. All other
///         // `#[cfg()]` attributes must be wrapped by `#[custom()]`.
///         #[cfg(feature = "external_forces")]
///         features:::ForcePlugin,
///         // More complicated `#[cfg()]`s and annotations are not supported by automatic doc
///         // generation, in which case you must wrap it in `#[custom()]`.
///         #[custom(cfg(target_arch = "wasm32"))]
///         web:::WebCompatibilityPlugin,
///         // You can nest `PluginGroup`s within other `PluginGroup`s, you just need the
///         // `#[plugin_group]` attribute.
///         #[plugin_group]
///         audio:::AudioPlugins,
///         // You can hide plugins from documentation. Due to macro limitations, hidden plugins
///         // must be last.
///         #[doc(hidden)]
///         internal:::InternalPlugin
///     }
///     /// You may add doc comments after the plugin group as well. They will be appended after
///     /// the documented list of plugins.
/// }
/// ```
#[macro_export]
macro_rules! plugin_group {
    {
        $(#[$group_meta:meta])*
        $vis:vis struct $group:ident {
            $(
                $(#[cfg(feature = $plugin_feature:literal)])?
                $(#[custom($plugin_meta:meta)])*
                $($plugin_path:ident::)* : $plugin_name:ident
            ),*
            $(
                $(,)?$(
                    #[plugin_group]
                    $(#[cfg(feature = $plugin_group_feature:literal)])?
                    $(#[custom($plugin_group_meta:meta)])*
                    $($plugin_group_path:ident::)* : $plugin_group_name:ident
                ),+
            )?
            $(
                $(,)?$(
                    #[doc(hidden)]
                    $(#[cfg(feature = $hidden_plugin_feature:literal)])?
                    $(#[custom($hidden_plugin_meta:meta)])*
                    $($hidden_plugin_path:ident::)* : $hidden_plugin_name:ident
                ),+
            )?

            $(,)?
        }
        $($(#[doc = $post_doc:literal])+)?
    } => {
        $(#[$group_meta])*
        ///
        $(#[doc = concat!(
            " - [`", stringify!($plugin_name), "`](" $(, stringify!($plugin_path), "::")*, stringify!($plugin_name), ")"
            $(, " - with feature `", $plugin_feature, "`")?
        )])*
       $($(#[doc = concat!(
            " - [`", stringify!($plugin_group_name), "`](" $(, stringify!($plugin_group_path), "::")*, stringify!($plugin_group_name), ")"
            $(, " - with feature `", $plugin_group_feature, "`")?
        )])+)?
        $(
            ///
            $(#[doc = $post_doc])+
        )?
        $vis struct $group;

        impl $crate::PluginGroup for $group {
            fn build(self) -> $crate::PluginGroupBuilder {
                let mut group = $crate::PluginGroupBuilder::start::<Self>();

                $(
                    $(#[cfg(feature = $plugin_feature)])?
                    $(#[$plugin_meta])*
                    {
                        const _: () = {
                            const fn check_default<T: Default>() {}
                            check_default::<$($plugin_path::)*$plugin_name>();
                        };

                        group = group.add(<$($plugin_path::)*$plugin_name>::default());
                    }
                )*
                $($(
                    $(#[cfg(feature = $plugin_group_feature)])?
                    $(#[$plugin_group_meta])*
                    {
                        const _: () = {
                            const fn check_default<T: Default>() {}
                            check_default::<$($plugin_group_path::)*$plugin_group_name>();
                        };

                        group = group.add_group(<$($plugin_group_path::)*$plugin_group_name>::default());
                    }
                )+)?
                $($(
                    $(#[cfg(feature = $hidden_plugin_feature)])?
                    $(#[$hidden_plugin_meta])*
                    {
                        const _: () = {
                            const fn check_default<T: Default>() {}
                            check_default::<$($hidden_plugin_path::)*$hidden_plugin_name>();
                        };

                        group = group.add(<$($hidden_plugin_path::)*$hidden_plugin_name>::default());
                    }
                )+)?

                group
            }
        }
    };
}

/// Combines multiple [`Plugin`]s into a single unit.
///
/// If you want an easier, but slightly more restrictive, method of implementing this trait, you
/// may be interested in the [`plugin_group!`] macro.
pub trait PluginGroup: Sized {
    /// Configures the [`Plugin`]s that are to be added.
    fn build(self) -> PluginGroupBuilder;
    /// Configures a name for the [`PluginGroup`] which is primarily used for debugging.
    fn name() -> String {
        core::any::type_name::<Self>().to_string()
    }
    /// Sets the value of the given [`Plugin`], if it exists
    fn set<T: Plugin>(self, plugin: T) -> PluginGroupBuilder {
        self.build().set(plugin)
    }
}

struct PluginEntry {
    plugin: Box<dyn Plugin>,
    enabled: bool,
}

impl PluginGroup for PluginGroupBuilder {
    fn build(self) -> PluginGroupBuilder {
        self
    }
}

/// Facilitates the creation and configuration of a [`PluginGroup`].
///
/// Provides a build ordering to ensure that [`Plugin`]s which produce/require a [`Resource`](bevy_ecs::resource::Resource)
/// are built before/after dependent/depending [`Plugin`]s. [`Plugin`]s inside the group
/// can be disabled, enabled or reordered.
pub struct PluginGroupBuilder {
    group_name: String,
    plugins: TypeIdMap<PluginEntry>,
    order: Vec<TypeId>,
}

impl PluginGroupBuilder {
    /// Start a new builder for the [`PluginGroup`].
    pub fn start<PG: PluginGroup>() -> Self {
        Self {
            group_name: PG::name(),
            plugins: Default::default(),
            order: Default::default(),
        }
    }

    /// Checks if the [`PluginGroupBuilder`] contains the given [`Plugin`].
    pub fn contains<T: Plugin>(&self) -> bool {
        self.plugins.contains_key(&TypeId::of::<T>())
    }

    /// Returns `true` if the [`PluginGroupBuilder`] contains the given [`Plugin`] and it's enabled.
    pub fn enabled<T: Plugin>(&self) -> bool {
        self.plugins
            .get(&TypeId::of::<T>())
            .is_some_and(|e| e.enabled)
    }

    /// Finds the index of a target [`Plugin`].
    fn index_of<Target: Plugin>(&self) -> Option<usize> {
        self.order
            .iter()
            .position(|&ty| ty == TypeId::of::<Target>())
    }

    // Insert the new plugin as enabled, and removes its previous ordering if it was
    // already present
    fn upsert_plugin_state<T: Plugin>(&mut self, plugin: T, added_at_index: usize) {
        self.upsert_plugin_entry_state(
            TypeId::of::<T>(),
            PluginEntry {
                plugin: Box::new(plugin),
                enabled: true,
            },
            added_at_index,
        );
    }

    // Insert the new plugin entry as enabled, and removes its previous ordering if it was
    // already present
    fn upsert_plugin_entry_state(
        &mut self,
        key: TypeId,
        plugin: PluginEntry,
        added_at_index: usize,
    ) {
        if let Some(entry) = self.plugins.insert(key, plugin) {
            if entry.enabled {
                warn!(
                    "You are replacing plugin '{}' that was not disabled.",
                    entry.plugin.name()
                );
            }
            if let Some(to_remove) = self
                .order
                .iter()
                .enumerate()
                .find(|(i, ty)| *i != added_at_index && **ty == key)
                .map(|(i, _)| i)
            {
                self.order.remove(to_remove);
            }
        }
    }

    /// Sets the value of the given [`Plugin`], if it exists.
    ///
    /// # Panics
    ///
    /// Panics if the [`Plugin`] does not exist.
    pub fn set<T: Plugin>(self, plugin: T) -> Self {
        self.try_set(plugin).unwrap_or_else(|_| {
            panic!(
                "{} does not exist in this PluginGroup",
                core::any::type_name::<T>(),
            )
        })
    }

    /// Tries to set the value of the given [`Plugin`], if it exists.
    ///
    /// If the given plugin doesn't exist returns self and the passed in [`Plugin`].
    pub fn try_set<T: Plugin>(mut self, plugin: T) -> Result<Self, (Self, T)> {
        match self.plugins.entry(TypeId::of::<T>()) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().plugin = Box::new(plugin);

                Ok(self)
            }
            Entry::Vacant(_) => Err((self, plugin)),
        }
    }

    /// Adds the plugin [`Plugin`] at the end of this [`PluginGroupBuilder`]. If the plugin was
    /// already in the group, it is removed from its previous place.
    // This is not confusing, clippy!
    #[expect(
        clippy::should_implement_trait,
        reason = "This does not emulate the `+` operator, but is more akin to pushing to a stack."
    )]
    pub fn add<T: Plugin>(mut self, plugin: T) -> Self {
        let target_index = self.order.len();
        self.order.push(TypeId::of::<T>());
        self.upsert_plugin_state(plugin, target_index);
        self
    }

    /// Attempts to add the plugin [`Plugin`] at the end of this [`PluginGroupBuilder`].
    ///
    /// If the plugin was already in the group the addition fails.
    pub fn try_add<T: Plugin>(self, plugin: T) -> Result<Self, (Self, T)> {
        if self.contains::<T>() {
            return Err((self, plugin));
        }

        Ok(self.add(plugin))
    }

    /// Adds a [`PluginGroup`] at the end of this [`PluginGroupBuilder`]. If the plugin was
    /// already in the group, it is removed from its previous place.
    pub fn add_group(mut self, group: impl PluginGroup) -> Self {
        let Self {
            mut plugins, order, ..
        } = group.build();

        for plugin_id in order {
            self.upsert_plugin_entry_state(
                plugin_id,
                plugins.remove(&plugin_id).unwrap(),
                self.order.len(),
            );

            self.order.push(plugin_id);
        }

        self
    }

    /// Adds a [`Plugin`] in this [`PluginGroupBuilder`] before the plugin of type `Target`.
    ///
    /// If the plugin was already the group, it is removed from its previous place.
    ///
    /// # Panics
    ///
    /// Panics if `Target` is not already in this [`PluginGroupBuilder`].
    pub fn add_before<Target: Plugin>(self, plugin: impl Plugin) -> Self {
        self.try_add_before_overwrite::<Target, _>(plugin)
            .unwrap_or_else(|_| {
                panic!(
                    "Plugin does not exist in group: {}.",
                    core::any::type_name::<Target>()
                )
            })
    }

    /// Adds a [`Plugin`] in this [`PluginGroupBuilder`] before the plugin of type `Target`.
    ///
    /// If the plugin was already in the group the add fails. If there isn't a plugin
    /// of type `Target` in the group the plugin we're trying to insert is returned.
    pub fn try_add_before<Target: Plugin, Insert: Plugin>(
        self,
        plugin: Insert,
    ) -> Result<Self, (Self, Insert)> {
        if self.contains::<Insert>() {
            return Err((self, plugin));
        }

        self.try_add_before_overwrite::<Target, _>(plugin)
    }

    /// Adds a [`Plugin`] in this [`PluginGroupBuilder`] before the plugin of type `Target`.
    ///
    /// If the plugin was already in the group, it is removed from its previous places.
    /// If there isn't a plugin of type `Target` in the group the plugin we're trying to insert
    /// is returned.
    pub fn try_add_before_overwrite<Target: Plugin, Insert: Plugin>(
        mut self,
        plugin: Insert,
    ) -> Result<Self, (Self, Insert)> {
        let Some(target_index) = self.index_of::<Target>() else {
            return Err((self, plugin));
        };

        self.order.insert(target_index, TypeId::of::<Insert>());
        self.upsert_plugin_state(plugin, target_index);
        Ok(self)
    }

    /// Adds a [`Plugin`] in this [`PluginGroupBuilder`] after the plugin of type `Target`.
    ///
    /// If the plugin was already the group, it is removed from its previous place.
    ///
    /// # Panics
    ///
    /// Panics if `Target` is not already in this [`PluginGroupBuilder`].
    pub fn add_after<Target: Plugin>(self, plugin: impl Plugin) -> Self {
        self.try_add_after_overwrite::<Target, _>(plugin)
            .unwrap_or_else(|_| {
                panic!(
                    "Plugin does not exist in group: {}.",
                    core::any::type_name::<Target>()
                )
            })
    }

    /// Adds a [`Plugin`] in this [`PluginGroupBuilder`] after the plugin of type `Target`.
    ///
    /// If the plugin was already in the group the add fails. If there isn't a plugin
    /// of type `Target` in the group the plugin we're trying to insert is returned.
    pub fn try_add_after<Target: Plugin, Insert: Plugin>(
        self,
        plugin: Insert,
    ) -> Result<Self, (Self, Insert)> {
        if self.contains::<Insert>() {
            return Err((self, plugin));
        }

        self.try_add_after_overwrite::<Target, _>(plugin)
    }

    /// Adds a [`Plugin`] in this [`PluginGroupBuilder`] after the plugin of type `Target`.
    ///
    /// If the plugin was already in the group, it is removed from its previous places.
    /// If there isn't a plugin of type `Target` in the group the plugin we're trying to insert
    /// is returned.
    pub fn try_add_after_overwrite<Target: Plugin, Insert: Plugin>(
        mut self,
        plugin: Insert,
    ) -> Result<Self, (Self, Insert)> {
        let Some(target_index) = self.index_of::<Target>() else {
            return Err((self, plugin));
        };

        let target_index = target_index + 1;

        self.order.insert(target_index, TypeId::of::<Insert>());
        self.upsert_plugin_state(plugin, target_index);
        Ok(self)
    }

    /// Enables a [`Plugin`].
    ///
    /// [`Plugin`]s within a [`PluginGroup`] are enabled by default. This function is used to
    /// opt back in to a [`Plugin`] after [disabling](Self::disable) it. If there are no plugins
    /// of type `T` in this group, it will panic.
    pub fn enable<T: Plugin>(mut self) -> Self {
        let plugin_entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot enable a plugin that does not exist.");
        plugin_entry.enabled = true;
        self
    }

    /// Disables a [`Plugin`], preventing it from being added to the [`App`] with the rest of the
    /// [`PluginGroup`]. The disabled [`Plugin`] keeps its place in the [`PluginGroup`], so it can
    /// still be used for ordering with [`add_before`](Self::add_before) or
    /// [`add_after`](Self::add_after), or it can be [re-enabled](Self::enable). If there are no
    /// plugins of type `T` in this group, it will panic.
    pub fn disable<T: Plugin>(mut self) -> Self {
        let plugin_entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot disable a plugin that does not exist.");
        plugin_entry.enabled = false;
        self
    }

    /// Consumes the [`PluginGroupBuilder`] and [builds](Plugin::build) the contained [`Plugin`]s
    /// in the order specified.
    ///
    /// # Panics
    ///
    /// Panics if one of the plugin in the group was already added to the application.
    #[track_caller]
    pub fn finish(mut self, app: &mut App) {
        for ty in &self.order {
            if let Some(entry) = self.plugins.remove(ty)
                && entry.enabled
            {
                debug!("added plugin: {}", entry.plugin.name());
                if let Err(AppError::DuplicatePlugin { plugin_name }) =
                    app.add_boxed_plugin(entry.plugin)
                {
                    panic!(
                        "Error adding plugin {} in group {}: plugin was already added in application",
                        plugin_name,
                        self.group_name
                    );
                }
            }
        }
    }
}

/// A plugin group which doesn't do anything. Useful for examples:
/// ```
/// # use bevy_app::prelude::*;
/// use bevy_app::NoopPluginGroup as MinimalPlugins;
///
/// fn main(){
///     App::new().add_plugins(MinimalPlugins).run();
/// }
/// ```
#[doc(hidden)]
pub struct NoopPluginGroup;

impl PluginGroup for NoopPluginGroup {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use core::{any::TypeId, fmt::Debug};

    use super::PluginGroupBuilder;
    use crate::{App, NoopPluginGroup, Plugin, PluginGroup};

    #[derive(Default)]
    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, _: &mut App) {}
    }

    #[derive(Default)]
    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, _: &mut App) {}
    }

    #[derive(Default)]
    struct PluginC;
    impl Plugin for PluginC {
        fn build(&self, _: &mut App) {}
    }

    #[derive(PartialEq, Debug)]
    struct PluginWithData(u32);
    impl Plugin for PluginWithData {
        fn build(&self, _: &mut App) {}
    }

    fn get_plugin<T: Debug + 'static>(group: &PluginGroupBuilder, id: TypeId) -> &T {
        group.plugins[&id]
            .plugin
            .as_any()
            .downcast_ref::<T>()
            .unwrap()
    }

    #[test]
    fn contains() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB);

        assert!(group.contains::<PluginA>());
        assert!(!group.contains::<PluginC>());

        let group = group.disable::<PluginA>();

        assert!(group.enabled::<PluginB>());
        assert!(!group.enabled::<PluginA>());
    }

    #[test]
    fn basic_ordering() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB)
            .add(PluginC);

        assert_eq!(
            group.order,
            vec![
                TypeId::of::<PluginA>(),
                TypeId::of::<PluginB>(),
                TypeId::of::<PluginC>(),
            ]
        );
    }

    #[test]
    fn add_before() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB)
            .add_before::<PluginB>(PluginC);

        assert_eq!(
            group.order,
            vec![
                TypeId::of::<PluginA>(),
                TypeId::of::<PluginC>(),
                TypeId::of::<PluginB>(),
            ]
        );
    }

    #[test]
    fn try_add_before() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>().add(PluginA);

        let Ok(group) = group.try_add_before::<PluginA, _>(PluginC) else {
            panic!("PluginA wasn't in group");
        };

        assert_eq!(
            group.order,
            vec![TypeId::of::<PluginC>(), TypeId::of::<PluginA>(),]
        );

        assert!(group.try_add_before::<PluginA, _>(PluginC).is_err());
    }

    #[test]
    #[should_panic(
        expected = "Plugin does not exist in group: bevy_app::plugin_group::tests::PluginB."
    )]
    fn add_before_nonexistent() {
        PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add_before::<PluginB>(PluginC);
    }

    #[test]
    fn add_after() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB)
            .add_after::<PluginA>(PluginC);

        assert_eq!(
            group.order,
            vec![
                TypeId::of::<PluginA>(),
                TypeId::of::<PluginC>(),
                TypeId::of::<PluginB>(),
            ]
        );
    }

    #[test]
    fn try_add_after() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB);

        let Ok(group) = group.try_add_after::<PluginA, _>(PluginC) else {
            panic!("PluginA wasn't in group");
        };

        assert_eq!(
            group.order,
            vec![
                TypeId::of::<PluginA>(),
                TypeId::of::<PluginC>(),
                TypeId::of::<PluginB>(),
            ]
        );

        assert!(group.try_add_after::<PluginA, _>(PluginC).is_err());
    }

    #[test]
    #[should_panic(
        expected = "Plugin does not exist in group: bevy_app::plugin_group::tests::PluginB."
    )]
    fn add_after_nonexistent() {
        PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add_after::<PluginB>(PluginC);
    }

    #[test]
    fn add_overwrite() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginWithData(0x0F))
            .add(PluginC);

        let id = TypeId::of::<PluginWithData>();
        assert_eq!(
            get_plugin::<PluginWithData>(&group, id),
            &PluginWithData(0x0F)
        );

        let group = group.add(PluginWithData(0xA0));

        assert_eq!(
            get_plugin::<PluginWithData>(&group, id),
            &PluginWithData(0xA0)
        );
        assert_eq!(
            group.order,
            vec![
                TypeId::of::<PluginA>(),
                TypeId::of::<PluginC>(),
                TypeId::of::<PluginWithData>(),
            ]
        );

        let Ok(group) = group.try_add_before_overwrite::<PluginA, _>(PluginWithData(0x01)) else {
            panic!("PluginA wasn't in group");
        };
        assert_eq!(
            get_plugin::<PluginWithData>(&group, id),
            &PluginWithData(0x01)
        );
        assert_eq!(
            group.order,
            vec![
                TypeId::of::<PluginWithData>(),
                TypeId::of::<PluginA>(),
                TypeId::of::<PluginC>(),
            ]
        );

        let Ok(group) = group.try_add_after_overwrite::<PluginA, _>(PluginWithData(0xdeadbeef))
        else {
            panic!("PluginA wasn't in group");
        };
        assert_eq!(
            get_plugin::<PluginWithData>(&group, id),
            &PluginWithData(0xdeadbeef)
        );
        assert_eq!(
            group.order,
            vec![
                TypeId::of::<PluginA>(),
                TypeId::of::<PluginWithData>(),
                TypeId::of::<PluginC>(),
            ]
        );
    }

    #[test]
    fn readd() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB)
            .add(PluginC)
            .add(PluginB);

        assert_eq!(
            group.order,
            vec![
                TypeId::of::<PluginA>(),
                TypeId::of::<PluginC>(),
                TypeId::of::<PluginB>(),
            ]
        );
    }

    #[test]
    fn readd_before() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB)
            .add(PluginC)
            .add_before::<PluginB>(PluginC);

        assert_eq!(
            group.order,
            vec![
                TypeId::of::<PluginA>(),
                TypeId::of::<PluginC>(),
                TypeId::of::<PluginB>(),
            ]
        );
    }

    #[test]
    fn readd_after() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB)
            .add(PluginC)
            .add_after::<PluginA>(PluginC);

        assert_eq!(
            group.order,
            vec![
                TypeId::of::<PluginA>(),
                TypeId::of::<PluginC>(),
                TypeId::of::<PluginB>(),
            ]
        );
    }

    #[test]
    fn add_basic_subgroup() {
        let group_a = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB);

        let group_b = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add_group(group_a)
            .add(PluginC);

        assert_eq!(
            group_b.order,
            vec![
                TypeId::of::<PluginA>(),
                TypeId::of::<PluginB>(),
                TypeId::of::<PluginC>(),
            ]
        );
    }

    #[test]
    fn add_conflicting_subgroup() {
        let group_a = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginC);

        let group_b = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginB)
            .add(PluginC);

        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add_group(group_a)
            .add_group(group_b);

        assert_eq!(
            group.order,
            vec![
                TypeId::of::<PluginA>(),
                TypeId::of::<PluginB>(),
                TypeId::of::<PluginC>(),
            ]
        );
    }

    plugin_group! {
        #[derive(Default)]
        struct PluginGroupA {
            :PluginA
        }
    }
    plugin_group! {
        #[derive(Default)]
        struct PluginGroupB {
            :PluginB
        }
    }
    plugin_group! {
        struct PluginGroupC {
            :PluginC
            #[plugin_group]
            :PluginGroupA,
            #[plugin_group]
            :PluginGroupB,
        }
    }
    #[test]
    fn construct_nested_plugin_groups() {
        PluginGroupC {}.build();
    }
}
