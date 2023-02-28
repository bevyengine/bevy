use crate::{App, AppError, Plugin, PluginGroup};
use bevy_ecs::schedule::simple_cycles_in_component;
use bevy_utils::{
    default,
    petgraph::{algo::TarjanScc, prelude::DiGraphMap},
    tracing::{debug, error},
    HashMap,
};

use std::any::TypeId;
use std::fmt::Write;

#[derive(Default)]
pub(crate) struct PluginStore {
    plugins: HashMap<TypeId, Box<dyn Plugin>>,
    graph: DiGraphMap<TypeId, Edge>,
    subs: HashMap<TypeId, TypeId>,
}

enum Edge {
    DependencyOf,
    SubstituteOf,
}

impl PluginStore {
    pub(crate) fn new() -> Self {
        Self {
            plugins: default(),
            graph: default(),
            subs: default(),
        }
    }

    pub(crate) fn add<T>(&mut self, plugin: T) -> Result<(), AppError>
    where
        T: Plugin,
    {
        self.add_boxed(Box::new(plugin))
    }

    pub(crate) fn add_boxed(&mut self, plugin: Box<dyn Plugin>) -> Result<(), AppError> {
        let plugin_id = plugin.type_id();
        if self.plugins.contains_key(&plugin_id) {
            // plugin already exists
            return Err(AppError::PluginAlreadyExists(plugin.name().to_string()));
        }

        self.graph.add_node(plugin_id);
        for other_id in plugin.depends_on().into_iter() {
            self.graph.add_edge(other_id, plugin_id, Edge::DependencyOf);
        }

        for other_id in plugin.subs_for().into_iter() {
            if let Some(&subber_id) = self.subs.get(&other_id) {
                // substitution already exists
                let subber = self.plugins.get(&subber_id).unwrap();
                let subbed = self.plugins.get(&other_id).unwrap();

                return Err(AppError::PluginAlreadySubstituted {
                    plugin: plugin.name().to_string(),
                    subber: subber.name().to_string(),
                    subbed: subbed.name().to_string(),
                });
            }

            if let Some(edge) = self.graph.edge_weight(plugin_id, other_id) {
                // "depends on" relationship already exists
                if matches!(edge, Edge::DependencyOf) {
                    error!(
                        "{} both substitutes and depends on another plugin.",
                        plugin.name()
                    );
                    return Err(AppError::PluginDependencyCycle);
                }
            }

            self.graph.add_edge(plugin_id, other_id, Edge::SubstituteOf);
        }

        debug!("added plugin: {}", plugin.name());
        self.plugins.insert(plugin_id, plugin);

        Ok(())
    }

    pub(crate) fn add_group<T: PluginGroup>(&mut self, plugins: T) -> Result<(), AppError> {
        let group = plugins.build();
        for entry in group.plugins.into_values() {
            if entry.enabled {
                let plugin = entry.plugin;
                debug!("{}", group.group_name);
                self.add_boxed(plugin)?;
            }
        }

        Ok(())
    }

    pub(crate) fn contains<T>(&self) -> bool
    where
        T: Plugin,
    {
        self.plugins
            .values()
            .any(|p| p.downcast_ref::<T>().is_some())
    }

    pub(crate) fn get_plugins<T>(&self) -> Vec<&T>
    where
        T: Plugin,
    {
        self.plugins
            .values()
            .filter_map(|p| p.downcast_ref())
            .collect()
    }

    pub(crate) fn topological_sort(&self) -> Result<Vec<TypeId>, Vec<Vec<TypeId>>> {
        let n = self.graph.node_count();
        let mut sccs_with_cycles = Vec::with_capacity(n);
        let mut top_sorted_nodes = Vec::with_capacity(n);

        // topologically sort the dependency graph
        let mut tarjan_scc = TarjanScc::new();
        tarjan_scc.run(&self.graph, |scc| {
            if scc.len() > 1 {
                sccs_with_cycles.push(scc.to_vec());
            } else {
                top_sorted_nodes.extend_from_slice(&scc);
            }
        });

        // must reverse to get topological order
        sccs_with_cycles.reverse();
        top_sorted_nodes.reverse();

        if sccs_with_cycles.is_empty() {
            Ok(top_sorted_nodes)
        } else {
            Err(sccs_with_cycles)
        }
    }

    pub(crate) fn try_order_plugins(&self) -> Result<Vec<TypeId>, AppError> {
        let mut cycles = Vec::new();
        // self-loops
        for id in self.plugins.keys().cloned() {
            if self.graph.contains_edge(id, id) {
                cycles.push(vec![id, id]);
            }
        }

        let build_order = match self.topological_sort() {
            Ok(top_sorted_nodes) => top_sorted_nodes,
            Err(sccs_with_cycles) => {
                for scc in sccs_with_cycles.into_iter() {
                    cycles.append(&mut simple_cycles_in_component(&self.graph, &scc));
                }

                Vec::new()
            }
        };

        if !cycles.is_empty() {
            let mut message = format!("{} cyclic plugin dependencies found:\n", cycles.len());
            for (i, cycle) in cycles.into_iter().enumerate() {
                let mut iter = cycle.into_iter();

                let first = iter.next().unwrap();
                let first_name = self.plugins[&first].name();

                let mut prev = first;
                let lines = iter.chain(std::iter::once(first)).map(|curr| {
                    let name = self.plugins[&curr].name();
                    let relation_with = match self.graph.edge_weight(prev, curr).unwrap() {
                        Edge::DependencyOf => "is depended on by",
                        Edge::SubstituteOf => "substitutes",
                    };
                    prev = curr;

                    (relation_with, name)
                });

                writeln!(
                    message,
                    "cycle {}: plugin '{first_name}' depends on itself",
                    i + 1,
                )
                .unwrap();
                writeln!(message, "'{first_name}'").unwrap();
                for (relation_with, name) in lines {
                    writeln!(message, " ... which {relation_with} plugin '{name}'").unwrap();
                }
                writeln!(message).unwrap();

                error!("{}", message);
                return Err(AppError::PluginDependencyCycle);
            }
        }

        Ok(build_order)
    }

    pub(crate) fn build(&mut self, app: &mut App) -> Result<(), AppError> {
        // sort graph (or report cycles)
        let build_order = self.try_order_plugins()?;

        // build plugins
        for plugin_id in &build_order {
            let plugin = self.plugins.get_mut(plugin_id).unwrap();
            if self.subs.contains_key(plugin_id) {
                continue;
            }

            debug!("building plugin '{}'", plugin.name());
            plugin.build(app);
        }

        // setup plugins
        for plugin_id in &build_order {
            let plugin = self.plugins.get_mut(plugin_id).unwrap();
            if self.subs.contains_key(plugin_id) {
                continue;
            }

            debug!("setting up plugin '{}'", plugin.name());
            plugin.setup(app);
        }

        self.plugins.clear();
        self.graph.clear();
        self.subs.clear();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{App, NoopPluginGroup, Plugin, PluginGroupBuilder, PluginStore};
    use std::any::TypeId;

    struct SubsForSelf;
    impl Plugin for SubsForSelf {
        fn build(&self, _: &mut App) {}

        fn subs_for(&self) -> Vec<TypeId> {
            vec![TypeId::of::<SubsForSelf>()]
        }
    }

    #[test]
    fn add_different_plugins() {
        struct A;
        impl Plugin for A {
            fn build(&self, _: &mut App) {}
        }

        struct B;
        impl Plugin for B {
            fn build(&self, _: &mut App) {}
        }

        let mut store = PluginStore::new();
        assert!(store.add(A).is_ok());
        assert!(store.add(B).is_ok());
    }

    #[test]
    fn add_same_plugin_twice() {
        struct P;
        impl Plugin for P {
            fn build(&self, _: &mut App) {}
        }

        let mut store = PluginStore::new();
        assert!(store.add(P).is_ok());
        assert!(store.add(P).is_err());
    }

    #[test]
    fn basic_ordering() {
        struct A;
        impl Plugin for A {
            fn build(&self, _: &mut App) {}
        }

        struct B;
        impl Plugin for B {
            fn build(&self, _: &mut App) {}
            fn depends_on(&self) -> Vec<TypeId> {
                vec![TypeId::of::<A>()]
            }
        }

        struct C;
        impl Plugin for C {
            fn build(&self, _: &mut App) {}
            fn depends_on(&self) -> Vec<TypeId> {
                vec![TypeId::of::<B>()]
            }
        }

        let mut store = PluginStore::new();
        store.add(B).unwrap();
        store.add(C).unwrap();
        store.add(A).unwrap();

        assert_eq!(
            store.try_order_plugins().unwrap(),
            vec![TypeId::of::<A>(), TypeId::of::<B>(), TypeId::of::<C>()]
        );
    }

    #[test]
    fn group_ordering() {
        struct A;
        impl Plugin for A {
            fn build(&self, _: &mut App) {}
        }

        struct B;
        impl Plugin for B {
            fn build(&self, _: &mut App) {}
            fn depends_on(&self) -> Vec<TypeId> {
                vec![TypeId::of::<A>()]
            }
        }

        struct C;
        impl Plugin for C {
            fn build(&self, _: &mut App) {}
            fn depends_on(&self) -> Vec<TypeId> {
                vec![TypeId::of::<B>()]
            }
        }

        let group = PluginGroupBuilder::new::<NoopPluginGroup>()
            .add(C)
            .add(A)
            .add(B);

        let mut store = PluginStore::new();
        store.add_group(group).unwrap();

        assert_eq!(
            store.try_order_plugins().unwrap(),
            vec![TypeId::of::<A>(), TypeId::of::<B>(), TypeId::of::<C>()]
        );
    }

    #[test]
    fn plugin_depends_on_itself() {
        struct DependsOnItself;
        impl Plugin for DependsOnItself {
            fn build(&self, _: &mut App) {}
            fn depends_on(&self) -> Vec<TypeId> {
                vec![TypeId::of::<DependsOnItself>()]
            }
        }

        let mut store = PluginStore::new();
        store.add(DependsOnItself).unwrap();
        assert!(store.try_order_plugins().is_err());
    }

    #[test]
    fn plugin_substitutes_itself() {
        struct SubsForItself;
        impl Plugin for SubsForItself {
            fn build(&self, _: &mut App) {}
            fn subs_for(&self) -> Vec<TypeId> {
                vec![TypeId::of::<SubsForItself>()]
            }
        }

        let mut store = PluginStore::new();
        store.add(SubsForItself).unwrap();
        assert!(store.try_order_plugins().is_err());
    }

    #[test]
    fn simple_cycle() {
        struct A;
        impl Plugin for A {
            fn build(&self, _: &mut App) {}
            fn depends_on(&self) -> Vec<TypeId> {
                vec![TypeId::of::<C>()]
            }
        }

        struct B;
        impl Plugin for B {
            fn build(&self, _: &mut App) {}
            fn depends_on(&self) -> Vec<TypeId> {
                vec![TypeId::of::<A>()]
            }
        }

        struct C;
        impl Plugin for C {
            fn build(&self, _: &mut App) {}
            fn depends_on(&self) -> Vec<TypeId> {
                vec![TypeId::of::<B>()]
            }
        }

        let mut store = PluginStore::new();
        store.add(A).unwrap();
        store.add(B).unwrap();
        store.add(C).unwrap();

        assert!(store.try_order_plugins().is_err());
    }

    #[test]
    fn complex_cycle() {
        struct A;
        impl Plugin for A {
            fn build(&self, _: &mut App) {}
        }

        struct B;
        impl Plugin for B {
            fn build(&self, _: &mut App) {}
            fn depends_on(&self) -> Vec<TypeId> {
                vec![TypeId::of::<A>()]
            }
        }

        struct C;
        impl Plugin for C {
            fn build(&self, _: &mut App) {}
            fn depends_on(&self) -> Vec<TypeId> {
                vec![TypeId::of::<B>()]
            }
            fn subs_for(&self) -> Vec<TypeId> {
                vec![TypeId::of::<A>()]
            }
        }

        let mut store = PluginStore::new();
        store.add(A).unwrap();
        store.add(B).unwrap();
        store.add(C).unwrap();

        assert!(store.try_order_plugins().is_err());
    }
}
