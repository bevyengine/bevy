use bevy_utils::{
    petgraph::{algo::TarjanScc, graphmap::DiGraphMap},
    thiserror::Error,
    HashMap, HashSet,
};
use downcast_rs::{impl_downcast, Downcast};

use crate::App;
use std::any::{Any, TypeId};

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
/// * all instances of the plugin configure the shared manifest [`Plugin::configure`]
/// * on startup the app orders the plugins and calls [`Plugin::build`]
/// * the app then waits for all registered [`Plugin::ready`] to return `true`
/// * then it calls all registered [`Plugin::finish`]
/// * and finally it calls all registered [`Plugin::cleanup`]
pub trait Plugin: Downcast + Any + Send + Sync {
    /// Configures the [`App`] to which this plugin is added.
    fn build(&self, app: &mut App);

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

    /// If the plugin can be meaningfully instantiated several times in an [`App`],
    /// override this method to return `false`.
    fn is_unique(&self) -> bool {
        true
    }

    /// Provides information about how the plugin relates to other plugins. Override this function to
    /// specify dependencies, or allow this plugin to substitute for others. Changes applied here are
    /// common to all instances of the plugin.
    fn configure(&self, _manifest: &mut PluginManifest) {
        // do nothing
    }
}

impl_downcast!(Plugin);

/// A relationship one [`Plugin`] has with another.
///
/// Relations between plugins are used to track things plugin dependencies, and are the core
/// abstraction around which the [`PluginManifest`] is built around.
///
/// Note to maintainers: When adding a new relation, be sure to implement a function on
/// [`PluginManifest`] to expose it to users.
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub(crate) enum PluginRelation {
    /// Indicates this plugin must be built after the other one. The value indicates whether the
    /// the app should panic if the target plugin is not found.
    After(bool),
    /// Indicates that this plugin can be substituted for the other one.
    SubstituteFor,
}

impl PluginRelation {
    /// Tries to combine two [`PluginRelation`]s. This determines the outcome when multiple relations
    /// are added to the same plugin.
    fn combine(self, other: PluginRelation) -> Result<Self, (PluginRelation, PluginRelation)> {
        use PluginRelation::*;
        match (self, other) {
            // Two dependencies on the same plugin are compatible, and required if either is required.
            (After(first_required), After(second_required)) => {
                Ok(After(first_required | second_required))
            }
            // Two substitutions for the same plugin are equivalent.
            (SubstituteFor, SubstituteFor) => Ok(SubstituteFor),
            // All other combinations conflict.
            conflict => Err(conflict),
        }
    }
}

/// A description of a relationship with another plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginManifestEntry {
    /// The relationship with the target plugin.
    pub(crate) relation: PluginRelation,
    /// The type name of the target plugin. We must track this to produce friendly error messages.
    pub(crate) plugin_name: &'static str,
}

/// A plugin manifest specifies relationships with other plugins.
///
/// # Use
/// The primary use for a manifest is in [`Plugin::configure`]. Plugins which implement this function
/// will be passed a reference to a manifest when they are added to the app. The manifest can then
/// be used to specify dependencies (using [`PluginManifest::add_dependency`]) and substitutions
/// (using [`PluginManifest::add_substitution`].
///
/// *Note:* All instances of a non-unique plugin use and write to same manifest. Different instances
/// may specify different dependencies, but they are not allowed to contradict one another.
///
/// # Internal Details
/// The manifest is built around the [`PluginRelations`] type, and relies on the following assumptions:
/// + A relation is specified by the [`TypeId`] of the target plugin type.
/// + A relation applies to all instances of the target plugin.
/// + Relations are mutually exclusive; Plugins cannot have two different relations with the same target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManifest {
    /// The type name of the plugin. We must track this to produce friendly error messages.
    pub(crate) name: &'static str,
    /// The collection of relations.
    pub(crate) entries: HashMap<TypeId, PluginManifestEntry>,
}

impl PluginManifest {
    fn new<P: Plugin>() -> PluginManifest {
        PluginManifest {
            name: std::any::type_name::<P>(),
            entries: HashMap::new(),
        }
    }

    /// Adds an entry to the manifest. Returns an error if the new relation conflicts with an existing one.
    fn add_entry(
        &mut self,
        plugin_id: TypeId,
        new_entry: PluginManifestEntry,
    ) -> Result<(), PluginError> {
        match self.entries.get_mut(&plugin_id) {
            None => {
                self.entries.insert(plugin_id, new_entry);
            }
            Some(existing_entry) => match existing_entry.relation.combine(new_entry.relation) {
                Ok(relation) => existing_entry.relation = relation,
                Err((existing, new)) => {
                    return Err(PluginError::RelationConflict {
                        plugin_name: self.name,
                        target_name: existing_entry.plugin_name,
                        existing,
                        new,
                    })
                }
            },
        };
        Ok(())
    }

    /// Adds a relation to the specified plugin type.
    fn add_relation<P: Plugin>(&mut self, relation: PluginRelation) {
        let plugin_id = TypeId::of::<P>();
        let plugin_name = std::any::type_name::<P>();
        let entry = PluginManifestEntry {
            plugin_name,
            relation,
        };
        if let Err(PluginError::RelationConflict { existing, new, .. }) =
            self.add_entry(plugin_id, entry)
        {
            panic!(
                "Existing relation '{:?}' on '{}' conflicts with new relation '{:?}'",
                existing, plugin_name, new
            )
        }
    }

    // PUBLIC API -------------------------------------------------------------

    /// Adds a dependency upon the specified [`Plugin`]. If `required` is true and the dependency
    /// is not provided to the [`App`], it will panic. Otherwise it will emit a warning.
    ///
    /// # Panics
    ///
    /// Adding both an optional and a required dependency to the same plugin will panic. Adding both
    /// a dependency on a substituted plugin also panic.
    pub fn add_dependency<P: Plugin>(&mut self, required: bool) {
        self.add_relation::<P>(PluginRelation::After(required))
    }

    /// Tells the [`App`] to replace the specified [`Plugin`] with the plugin this manifest belongs to.
    ///
    /// # Panics
    ///
    /// Panics if a substitution is added for a dependency.
    pub fn add_substitution<P: Plugin>(&mut self) {
        self.add_relation::<P>(PluginRelation::SubstituteFor)
    }
}

/// Groups multiple compatible instances of the same plugin with information about the plugin type.
///
/// PluginFamilies are meant to be valid by construction. This means that a single family cannot
/// contain both unique and non-unique versions of the same plugin.
pub(crate) struct PluginFamily {
    /// A list of one or more boxed instances of the plugin, in arbitrary order. This is only empty
    /// when a family is first created.
    ///
    /// Note to maintainers: Rustc requires us to box each of these individually, but all plugins in
    /// this vector *must* have the correct type, or it will totally break error reporting.
    pub(crate) plugins: Vec<Box<dyn Plugin>>,
    /// True if the plugin is unique, in which case `plugins` should be of length one.
    pub(crate) is_unique: bool,
    /// Disables building this plugin when false.
    pub(crate) is_enabled: bool,
    /// The manifest for this plugin type.
    pub(crate) manifest: PluginManifest,
}

impl PluginFamily {
    /// Creates an empty plugin family. A new member should be added to this immediately.
    fn empty<P: Plugin>() -> PluginFamily {
        PluginFamily {
            plugins: Vec::new(),
            is_unique: false,
            is_enabled: true,
            manifest: PluginManifest::new::<P>(),
        }
    }
}

impl PluginFamily {
    /// Adds a single boxed plugin to the family.
    fn add(&mut self, plugin: Box<dyn Plugin>) -> Result<(), PluginError> {
        if plugin.is_unique() && self.plugins.is_empty() {
            // A unique plugin can only be added if no other instances have been added.
            plugin.configure(&mut self.manifest);
            self.plugins.push(plugin);
            self.is_unique = true;
            return Ok(());
        } else if !plugin.is_unique() && !self.is_unique {
            // A non-unique plugin can only be added when no unique instance has been added.
            plugin.configure(&mut self.manifest);
            self.plugins.push(plugin);
            return Ok(());
        } else {
            return Err(PluginError::DuplicatePlugin {
                plugin_name: self.manifest.name,
            });
        }
    }

    /// Merges another instance of the same plugin family into this one. Maintainers must ensure
    /// this is only called on families with matching plugin types.
    fn merge(&mut self, mut other: PluginFamily) -> Result<(), PluginError> {
        if self.is_unique == other.is_unique && (self.plugins.is_empty() || !self.is_unique) {
            // Uniqueness must match, and if unique this family must be empty.
            self.plugins.append(&mut other.plugins);
            for (plugin_id, entry) in other.manifest.entries {
                self.manifest.add_entry(plugin_id, entry)?;
            }
            // Disable the family if it is disabled in either set.
            self.is_enabled = self.is_enabled & other.is_enabled;
            Ok(())
        } else {
            return Err(PluginError::DuplicatePlugin {
                plugin_name: self.manifest.name,
            });
        }
    }
}

// The following three impls make it easy to map over all the plugins in a set using
// iters like `plugin_set.plugin_families.values().flatten()`.

impl IntoIterator for PluginFamily {
    type Item = Box<dyn Plugin>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.plugins.into_iter()
    }
}

impl<'a> IntoIterator for &'a PluginFamily {
    type Item = &'a Box<dyn Plugin>;
    type IntoIter = std::slice::Iter<'a, Box<dyn Plugin>>;

    fn into_iter(self) -> Self::IntoIter {
        self.plugins.iter()
    }
}

impl<'a> IntoIterator for &'a mut PluginFamily {
    type Item = &'a mut Box<dyn Plugin>;
    type IntoIter = std::slice::IterMut<'a, Box<dyn Plugin>>;

    fn into_iter(self) -> Self::IntoIter {
        self.plugins.iter_mut()
    }
}

/// Errors resulting from doing things with plugins. These may result from adding plugins to a set,
/// from combining sets, or from adding entries to the manifest in a [`Plugin::config`] function.
#[derive(Debug, Error)]
pub(crate) enum PluginError {
    /// A unique plugin was added multiple times.
    #[error("'{plugin_name:}' has already been added to the application.")]
    DuplicatePlugin {
        /// The name of the duplicated plugin.
        plugin_name: &'static str,
    },
    /// The manifest of a plugin contained conflicting relations with the same plugin.
    #[error(
        "'{plugin_name:}' has conflicting relations '{existing:?}' and '{new:?}' with '{target_name:}'."
    )]
    RelationConflict {
        /// The name of the plugin with the conflicting manifest.
        plugin_name: &'static str,
        /// The subject of the conflicting manifest declarations.
        target_name: &'static str,
        /// The existing relation declared by the plugin upon the target.
        existing: PluginRelation,
        /// The new (conflicting) relation declared by the plugin upon the target.
        new: PluginRelation,
    },
}

/// A common representation for a set of her erogenous [`Plugin`]s.
///
/// [`PluginSet::add_plugins`] can add plugins, tuples of plugins, [`PluginGroup`]s, and even other
/// `PluginSet`s to the set.
///
/// # Implementation Details
/// PluginSets track lots of useful information beyond simply holding boxed references to plugins themselves.
/// Plugin instances of the same type are all stored in a single [`PluginFamily`], along side their dependency
/// information (in the form of a [`PluginManifest`]) and static reference to the type name (for debug purposes).
///
/// The [`App`] uses a PluginSet internally to collect plugins before they are built. All plugins that are added
/// to the app eventually get merged into the app PluginSet.
#[derive(Default)]
pub struct PluginSet {
    pub(crate) plugin_families: HashMap<TypeId, PluginFamily>,
}

impl PluginSet {
    /// Add a new plugin to the set. This should only be called by types that implement [`Plugins`].
    /// Everything else should use the more general [`PluginSet::add_plugins`] instead.
    pub(crate) fn add_plugin<P: Plugin>(&mut self, plugin: P) -> Result<(), PluginError> {
        let plugin_id = plugin.type_id();
        self.plugin_families
            .entry(plugin_id)
            .or_insert_with(|| PluginFamily::empty::<P>())
            .add(Box::new(plugin))
    }

    // PUBLIC API -------------------------------------------------------------

    /// Creates an empty [`PluginSet`].
    pub fn new() -> PluginSet {
        PluginSet::default()
    }

    /// Adds [`Plugin`]s to the set. Supports individual [`Plugin`]s, tuples of plugins,
    /// [`PluginGroup`]s, and other [`PluginSet`]s.
    pub fn add_plugins<M>(&mut self, plugins: impl Plugins<M>) {
        plugins.add_to_set(self);
    }

    /// Makes the plugins in this set build after another plugin, when it is present.
    pub fn after<P: Plugin>(&mut self) {
        for family in self.plugin_families.values_mut() {
            family.manifest.add_dependency::<P>(false)
        }
    }

    /// Makes the plugins in this set depend on another plugin. The app will panic if the
    /// dependency is not supplied at startup.
    pub fn depends_on<P: Plugin>(&mut self) {
        for family in self.plugin_families.values_mut() {
            family.manifest.add_dependency::<P>(true)
        }
    }

    /// Disables the plugin if it is present in the set.
    pub fn disable<P: Plugin>(&mut self) {
        if let Some(family) = self.plugin_families.get_mut(&TypeId::of::<P>()) {
            family.is_enabled = false;
        }
    }

    /// Disables all the plugins in the set.
    pub fn disable_all(&mut self) {
        for family in self.plugin_families.values_mut() {
            family.is_enabled = false;
        }
    }
}

/// Types that implement this trait can be used to add [`Plugin`]s to a [`PluginSet`]. This is mostly used
/// to give names and types to specific sets of plugins.
///
/// # Examples
///
/// ```
/// # use bevy_app::{App, Plugin, PluginSet, PluginGroup, NoopPluginGroup as MinimalPlugins};
/// # struct MyPlugin;
/// # impl Plugin for MyPlugin {
/// #     fn build(&self, app: &mut App) {}
/// # }
/// struct MyPluginGroup;
///
/// impl PluginGroup for MyPluginGroup {
///     fn build(self, set: &mut PluginSet) {
///         set.add_plugins(MyPlugin);
///     }
/// }
///
/// fn main() {
///     App::new().add_plugins((MinimalPlugins, MyPluginGroup)).run();
/// }
/// ```
pub trait PluginGroup: Sized {
    /// Add plugins to the [`PluginSet`] to which the group is added.
    fn build(self, set: &mut PluginSet);
}

/// A plugin group which doesn't do anything. Useful for examples:
/// ```
/// # use bevy_app::prelude::*;
/// use bevy_app::NoopPluginGroup as MinimalPlugins;
///
/// fn main() {
///     App::new().add_plugins(MinimalPlugins).run();
/// }
/// ```
#[doc(hidden)]
pub struct NoopPluginGroup;

impl PluginGroup for NoopPluginGroup {
    fn build(self, _set: &mut PluginSet) {}
}

/// A type representing an unsafe function that returns a mutable pointer to a [`Plugin`].
/// It is used for dynamically loading plugins.
///
/// See `bevy_dynamic_plugin/src/loader.rs#dynamically_load_plugin`.
pub type CreatePlugin = unsafe fn() -> *mut dyn Plugin;

/// Types that represent a set of [`Plugin`]s.
///
/// This is implemented for all types which implement [`Plugin`] or [`PluginGroup`], as well as
/// for [`PluginSet`]s and tuples over [`Plugin`]s.
pub trait Plugins<Marker>: sealed::Plugins<Marker> {
    /// Turn this collection of plugins into a [`PluginSet`].
    fn into_set(self) -> PluginSet {
        let mut set = PluginSet::new();
        self.add_to_set(&mut set);
        set
    }
}

impl<Marker, T> Plugins<Marker> for T where T: sealed::Plugins<Marker> {}

mod sealed {

    use bevy_ecs::all_tuples;

    use crate::{Plugin, PluginGroup, PluginSet};

    pub trait Plugins<Marker>: Sized {
        fn add_to_set(self, set: &mut PluginSet);
    }

    pub struct PluginMarker;
    pub struct PluginSetMarker;
    pub struct PluginGroupMarker;
    pub struct PluginsTupleMarker;

    impl<P: Plugin> Plugins<PluginMarker> for P {
        #[track_caller]
        fn add_to_set(self, set: &mut PluginSet) {
            if let Err(error) = set.add_plugin(self) {
                panic!("Error adding plugin to set: {error}")
            }
        }
    }

    impl<G: PluginGroup> Plugins<PluginGroupMarker> for G {
        #[track_caller]
        fn add_to_set(self, set: &mut PluginSet) {
            self.build(set);
        }
    }

    impl Plugins<PluginSetMarker> for PluginSet {
        #[track_caller]
        fn add_to_set(self, set: &mut PluginSet) {
            for (plugin_id, plugin_family) in self.plugin_families {
                if let Some(set_family) = set.plugin_families.get_mut(&plugin_id) {
                    if let Err(error) = set_family.merge(plugin_family) {
                        panic!("Error combining plugin sets: {error}")
                    }
                } else {
                    set.plugin_families.insert(plugin_id, plugin_family);
                }
            }
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
                fn add_to_set(self, set: &mut PluginSet) {
                    let ($($plugins,)*) = self;
                    $($plugins.add_to_set(set);)*
                }
            }
        }
    }

    all_tuples!(impl_plugins_tuples, 0, 15, P, S);
}

impl PluginSet {
    /// Return an ordering for the plugin set
    pub(crate) fn order(&self) -> Vec<TypeId> {
        // In the graph, 'a -> b' means 'a before b'.
        let mut graph = DiGraphMap::default();
        let mut substitutes = HashMap::new();
        let mut requires = HashMap::new();
        let mut disabled = HashSet::new();

        // Extract plugin information from manifests.
        for (plugin_id, plugin_family) in self.plugin_families.iter() {
            if !plugin_family.is_enabled {
                disabled.insert(plugin_id);
            }
            graph.add_node(*plugin_id);
            for (entry_id, entry) in plugin_family.manifest.entries.iter() {
                match entry.relation {
                    PluginRelation::After(required) => {
                        graph.add_edge(*entry_id, *plugin_id, entry.relation);
                        if required {
                            requires.entry(entry_id).or_insert(vec![]).push(plugin_id);
                        }
                    }
                    PluginRelation::SubstituteFor => {
                        graph.add_edge(*plugin_id, *entry_id, entry.relation);
                        if let Some(_substitute_id) = substitutes.insert(entry_id, plugin_id) {
                            panic!("Multiple plugins substituting for the same thing")
                        }
                    }
                };
            }
        }

        let n = graph.node_count();
        // Strongly connected components
        let mut sccs_with_cycles = Vec::with_capacity(n);
        // Top-sorted nodes
        let mut top_sorted_nodes = Vec::with_capacity(n);

        // Topologically sort the dependency graph
        let mut tarjan_scc = TarjanScc::new();
        tarjan_scc.run(&graph, |scc| {
            if scc.len() > 1 {
                sccs_with_cycles.push(scc.to_vec());
            } else {
                top_sorted_nodes.extend_from_slice(scc);
            }
        });

        // Must reverse to get topological order
        sccs_with_cycles.reverse();
        top_sorted_nodes.reverse();

        // This vector will hold the plugins that will actually be used, in topological order.
        let mut order = vec![];

        // Check dependencies and determine final order
        if sccs_with_cycles.is_empty() {
            // No cycles detected
            for plugin_id in top_sorted_nodes {
                // Skip substituted plugins regardless of if they are provided or not.
                if substitutes.contains_key(&plugin_id) {
                    continue;
                }
                // Try to add the plugin to the order
                let status;
                if self.plugin_families.contains_key(&plugin_id) {
                    if !disabled.contains(&plugin_id) {
                        // Plugin enabled, provided, and not substituted for, so add it
                        order.push(plugin_id);
                        continue;
                    } else {
                        status = "disabled";
                    }
                } else {
                    status = "missing";
                }
                // Plugin is either missing or disabled
                if let Some(_required_by) = requires.get(&plugin_id) {
                    panic!("Required plugin is {status}");
                }
            }
        } else {
            // Cycles are present
            panic!("Cycles detected in dependency graph")
            // TODO: Add propper error handling
        }

        return order;
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use crate::{
        App, Plugin, PluginGroup, PluginManifest, PluginManifestEntry, PluginRelation, PluginSet,
    };

    // Basic Tests for creating and manipulating PluginSets.

    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, _: &mut App) {}
    }

    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, _: &mut App) {}
    }

    struct PluginC;
    impl Plugin for PluginC {
        fn build(&self, _: &mut App) {}
    }

    struct GroupAB;
    impl PluginGroup for GroupAB {
        fn build(self, set: &mut PluginSet) {
            set.add_plugins((PluginA, PluginB));
        }
    }

    struct PluginUnique(bool);
    impl Plugin for PluginUnique {
        fn build(&self, _: &mut App) {}

        fn is_unique(&self) -> bool {
            self.0
        }
    }

    #[test]
    fn add_simple_plugin_to_set() {
        // Plugins can be added to sets.

        let mut set = PluginSet::new();
        set.add_plugins(PluginA);

        let ref family = set.plugin_families[&TypeId::of::<PluginA>()];
        assert!(family.is_unique == true);
        assert!(family.manifest == PluginManifest::new::<PluginA>());
        assert!(family.plugins.len() == 1);
    }

    #[test]
    fn add_tuple_to_set() {
        // Tuples can contain all the other plugin collections.

        let mut set = PluginSet::new();
        set.add_plugins((GroupAB, PluginC, PluginSet::new(), ()));

        // Verify A
        let ref family_a = set.plugin_families[&TypeId::of::<PluginA>()];
        assert!(family_a.is_unique == true);
        assert!(family_a.manifest == PluginManifest::new::<PluginA>());
        assert!(family_a.plugins.len() == 1);

        // Verify B
        let ref family_b = set.plugin_families[&TypeId::of::<PluginB>()];
        assert!(family_b.is_unique == true);
        assert!(family_b.manifest == PluginManifest::new::<PluginB>());
        assert!(family_b.plugins.len() == 1);

        // Verify C
        let ref family_c = set.plugin_families[&TypeId::of::<PluginC>()];
        assert!(family_c.is_unique == true);
        assert!(family_c.manifest == PluginManifest::new::<PluginC>());
        assert!(family_c.plugins.len() == 1);
    }

    #[test]
    fn combine_sets() {
        // Plugins should be preserved when combining sets.

        let mut set_ab = PluginSet::new();
        set_ab.add_plugins((PluginA, PluginB));

        let mut set = PluginSet::new();
        set.add_plugins((PluginC, set_ab));

        // Verify A
        let ref family_a = set.plugin_families[&TypeId::of::<PluginA>()];
        assert!(family_a.is_unique == true);
        assert!(family_a.manifest == PluginManifest::new::<PluginA>());
        assert!(family_a.plugins.len() == 1);

        // Verify B
        let ref family_b = set.plugin_families[&TypeId::of::<PluginB>()];
        assert!(family_b.is_unique == true);
        assert!(family_b.manifest == PluginManifest::new::<PluginB>());
        assert!(family_b.plugins.len() == 1);

        // Verify C
        let ref family_c = set.plugin_families[&TypeId::of::<PluginC>()];
        assert!(family_c.is_unique == true);
        assert!(family_c.manifest == PluginManifest::new::<PluginC>());
        assert!(family_c.plugins.len() == 1);
    }

    #[test]
    #[should_panic]
    fn add_multiple_unique() {
        // It should not be possible to add multiple versions of a unique plugin to a set.
        let mut set = PluginSet::new();
        set.add_plugins((PluginUnique(true), PluginUnique(true)));
    }

    #[test]
    fn add_multiple_non_unique() {
        // It should be possible to add multiple versions of a non-unique plugin to a set.
        let mut set = PluginSet::new();
        set.add_plugins((PluginUnique(false), PluginUnique(false)));

        let ref family = set.plugin_families[&TypeId::of::<PluginUnique>()];
        assert!(family.is_unique == false);
        assert!(family.manifest == PluginManifest::new::<PluginUnique>());
        assert!(family.plugins.len() == 2);
    }

    #[test]
    #[should_panic]
    fn add_non_unique_after_unique() {
        // It should not be possible to add a non-unique plugins to a set containing a unique versions of that plugin.
        let mut set = PluginSet::new();
        set.add_plugins((PluginUnique(true), PluginUnique(false), PluginUnique(false)));
    }

    #[test]
    #[should_panic]
    fn add_unique_after_non_unique() {
        // It should not be possible to add a unique plugin to a set containing non-unique versions of that plugin.
        let mut set = PluginSet::new();
        set.add_plugins((PluginUnique(false), PluginUnique(false), PluginUnique(true)));
    }

    // Tests involving plugin relations. These can be tricky, because in the case of a non-unique plugin, relation errors
    // can happen either during `Plugin::configure` or later when different sets are combined.

    struct ComplexPlugin;

    impl Plugin for ComplexPlugin {
        fn build(&self, _: &mut App) {}

        fn configure(&self, manifest: &mut PluginManifest) {
            manifest.add_dependency::<PluginA>(true);
            manifest.add_dependency::<PluginB>(false);
            manifest.add_substitution::<PluginC>();
        }
    }

    #[test]
    fn add_complex_plugin_to_set() {
        // Plugin sets are able to get the correct information from `Plugin::configure`.

        let mut set = PluginSet::new();
        set.add_plugins(ComplexPlugin);

        let correct_manifest = PluginManifest {
            name: "bevy_app::plugin::tests::ComplexPlugin",
            entries: [
                (
                    TypeId::of::<PluginA>(),
                    PluginManifestEntry {
                        relation: PluginRelation::After(true),
                        plugin_name: "bevy_app::plugin::tests::PluginA",
                    },
                ),
                (
                    TypeId::of::<PluginB>(),
                    PluginManifestEntry {
                        relation: PluginRelation::After(false),
                        plugin_name: "bevy_app::plugin::tests::PluginB",
                    },
                ),
                (
                    TypeId::of::<PluginC>(),
                    PluginManifestEntry {
                        relation: PluginRelation::SubstituteFor,
                        plugin_name: "bevy_app::plugin::tests::PluginC",
                    },
                ),
            ]
            .into(),
        };

        let ref family = set.plugin_families[&TypeId::of::<ComplexPlugin>()];
        assert!(family.is_unique == true);
        assert!(family.manifest == correct_manifest);
        assert!(family.plugins.len() == 1);
    }

    struct ConflictingPlugin;

    impl Plugin for ConflictingPlugin {
        fn build(&self, _: &mut App) {}

        fn configure(&self, manifest: &mut PluginManifest) {
            manifest.add_dependency::<PluginA>(true);
            manifest.add_substitution::<PluginA>();
        }
    }

    #[test]
    #[should_panic]
    fn add_conflicting_relations() {
        // Plugins are not allowed to depend on and substitute for the same plugin.
        let mut set = PluginSet::new();
        set.add_plugins(ConflictingPlugin);
    }

    struct ConflictingNonUniquePlugin(bool);

    impl Plugin for ConflictingNonUniquePlugin {
        fn build(&self, _: &mut App) {}

        fn is_unique(&self) -> bool {
            false
        }

        fn configure(&self, manifest: &mut PluginManifest) {
            if self.0 {
                manifest.add_dependency::<PluginA>(true);
            } else {
                manifest.add_substitution::<PluginA>();
            }
        }
    }

    #[test]
    #[should_panic]
    fn merge_conflicting_relations() {
        // Conflicting instances of a non-unique plugin should produce errors when sets are combined.
        let mut set_a = PluginSet::new();
        set_a.add_plugins(ConflictingNonUniquePlugin(true));
        let mut set_b = PluginSet::new();
        set_a.add_plugins(ConflictingNonUniquePlugin(false));
        let mut set = PluginSet::new();
        set.add_plugins((set_a, set_b))
    }

    struct DependencyUpgradePlugin(bool);

    impl Plugin for DependencyUpgradePlugin {
        fn build(&self, _: &mut App) {}

        fn is_unique(&self) -> bool {
            false
        }

        fn configure(&self, manifest: &mut PluginManifest) {
            manifest.add_dependency::<PluginA>(self.0);
        }
    }

    #[test]
    fn required_dependency_overrides_optional() {
        // A required dependency should override an optimal one without an error.
        let mut set = PluginSet::new();
        set.add_plugins((
            DependencyUpgradePlugin(false),
            DependencyUpgradePlugin(true),
        ));

        let correct_manifest = PluginManifest {
            name: "bevy_app::plugin::tests::ComplexPlugin",
            entries: [(
                TypeId::of::<PluginA>(),
                PluginManifestEntry {
                    relation: PluginRelation::After(true),
                    plugin_name: "bevy_app::plugin::tests::PluginA",
                },
            )]
            .into(),
        };

        let ref family = set.plugin_families[&TypeId::of::<DependencyUpgradePlugin>()];
        assert!(family.is_unique == false);
        assert!(
            family.manifest.entries[&TypeId::of::<PluginA>()].relation
                == PluginRelation::After(true)
        );
        assert!(family.plugins.len() == 2);
    }
}
