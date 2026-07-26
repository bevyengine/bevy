use bevy_platform::collections::HashMap;
use core::{convert::identity, hash::Hash};
use std::{boxed::Box, collections::VecDeque, vec::Vec};

use crate::{
    plugin_data::{MessageRegistration, PluginTypeId},
    DeclarativePlugin, PluginOutput,
};

/// A list of "entry point" plugins and their outputs. This gets expanded into a graph.
pub(crate) struct PluginList {
    nodes: Vec<(Box<dyn DeclarativePlugin>, PluginOutput)>,
}

impl PluginList {
    /// Expand the list of entry point plugins into a full graph. Ignores recurring ZSTs.
    pub(crate) fn expand(mut self) -> Result<PluginRegistrationGraph, ()> {
        let mut zst_already_expanded: HashMap<PluginTypeId, RegistrationId> = HashMap::new();
        let mut graph = PluginRegistrationGraph::new();
        for (_, output) in &mut self.nodes {
            // Mark entry points.
            output.is_entry_point = true;
        }
        let mut dependency_queue = VecDeque::new();
        for (item, output) in self.nodes {
            if output.is_zero_sized_optimizable
                && !zst_already_expanded.contains_key(&output.working_plugin)
            {
                let type_id = output.working_plugin;
                let (reg_id, dependencies) = graph.insert_node(item, output);
                dependency_queue.extend(dependencies.0.into_iter().map(|d| (reg_id, d)));
                zst_already_expanded.insert(type_id, reg_id);
            } else if !output.is_zero_sized_optimizable {
            }
        }
        // TODO: detect cycles in expansion + stop adding when "expanded enough" + solved.
        // mean moving this logic to the PluginRegistrationGraph building side.
        while let Some((from, dependency)) = dependency_queue.pop_front() {
            if !zst_already_expanded.contains_key(&dependency.type_id)
                && let Some((dyn_plugin, output_fn)) = dependency.data
            {
                let Some(output) = output_fn(dyn_plugin.as_ref()) else {
                    continue;
                };
                let plugin_id = output.working_plugin;
                let can_zst_optimize = output.is_zero_sized_optimizable;
                let (reg_id, dependencies) = graph.insert_node(dyn_plugin, output);
                graph.insert_edge(from, plugin_id);
                for dependency in dependencies.0 {
                    if let Some(dependency_source) = graph.get(reg_id)
                        && graph
                            .instances(dependency.type_id)
                            .filter_map(|existing_instance| {
                                let source_approval = dependency_source
                                    .output
                                    .plugin_approval
                                    .get(&dependency.type_id)?;
                                Some(source_approval.approves(&existing_instance.plugin_data))
                            })
                            .any(identity)
                    {
                        continue;
                    }
                    dependency_queue.push_back((reg_id, dependency));
                }
                if can_zst_optimize {
                    zst_already_expanded.insert(plugin_id, reg_id);
                }
            } else if dependency.data.is_none() {
                // TODO:
            }
        }
        Ok(graph)
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Hash, Clone, Copy)]
pub(crate) struct RegistrationId(usize, PluginTypeId);

impl RegistrationId {
    pub(crate) fn plugin_id(&self) -> PluginTypeId {
        self.1
    }
}

pub(crate) struct PluginNode {
    registration_id: RegistrationId,
    plugin_data: Box<dyn DeclarativePlugin>,
    output: PluginOutput<()>,
    distance_from_entry: Option<usize>,
}

pub(crate) struct PluginRegistrationGraph {
    registration_counter: usize,
    nodes: HashMap<PluginTypeId, Vec<PluginNode>>,
    dependency_edges: HashMap<RegistrationId, Vec<PluginTypeId>>,
}

impl PluginRegistrationGraph {
    fn new_id(&mut self, plugin_id: PluginTypeId) -> RegistrationId {
        let id = RegistrationId(self.registration_counter, plugin_id);
        self.registration_counter += 1;
        id
    }

    pub(crate) fn new() -> Self {
        Self {
            registration_counter: 0,
            nodes: HashMap::new(),
            dependency_edges: HashMap::new(),
        }
    }

    #[must_use]
    pub(crate) fn insert_node<D>(
        &mut self,
        plugin_data: Box<dyn DeclarativePlugin>,
        output: PluginOutput<D>,
    ) -> (RegistrationId, D) {
        let plugin_id = output.working_plugin;
        let registration_id = self.new_id(plugin_id);
        let (output, dependencies) = output.extract_dependencies();
        let distance_from_entry = if output.is_entry_point { Some(0) } else { None };
        let node = PluginNode {
            registration_id,
            plugin_data,
            output,
            distance_from_entry,
        };
        self.nodes.entry(plugin_id).or_default().push(node);
        (registration_id, dependencies)
    }

    pub(crate) fn conditional_insert_node<D>(
        plugin_data: Box<dyn DeclarativePlugin>,
        output: PluginOutput<D>,
    ) -> Result<(RegistrationId, D), (Box<dyn DeclarativePlugin>, PluginOutput<D>)> {
        unimplemented!()
    }

    ///
    pub(crate) fn data_already_represented<D>(
        &self,
        data: &Box<dyn DeclarativePlugin>,
        output: &PluginOutput<D>,
    ) -> bool {
        let plugin_id = output.working_plugin;

        if output.is_zero_sized_optimizable && self.nodes.contains_key(&plugin_id) {
            // The plugin is ZST and the author has agreed that plugin
            // registration can be optimized over this property.
            return true;
        }

        //

        unimplemented!()
    }

    pub(crate) fn get(&self, registration_id: RegistrationId) -> Option<&PluginNode> {
        self.instances(registration_id.plugin_id())
            .find(|node| node.registration_id == registration_id)
    }

    pub(crate) fn evaluate_plugin(
        &self,
        plugin_id: PluginTypeId,
        data: &Box<dyn DeclarativePlugin>,
    ) -> impl Iterator<Item = (RegistrationId, bool)> {
        self.has_approvals_for_plugin(plugin_id).map(move |plugin| {
            let id = plugin.registration_id;
            let result = plugin
                .output
                .plugin_approval
                .get(&plugin_id)
                .map(move |approval| approval.approves(data))
                .unwrap_or(true);
            (id, result)
        })
    }

    pub(crate) fn has_approvals_for_plugin(
        &self,
        plugin_id: PluginTypeId,
    ) -> impl Iterator<Item = &PluginNode> {
        self.all_nodes().filter_map(move |node| {
            let item = node.output.plugin_approval.get(&plugin_id)?;
            if item.trivially_true() {
                return None;
            }
            Some(node)
        })
    }

    pub(crate) fn all_nodes(&self) -> impl Iterator<Item = &PluginNode> {
        self.nodes.iter().flat_map(|(_, nodes)| nodes.iter())
    }

    pub(crate) fn entry_points(&self) -> impl Iterator<Item = &PluginNode> {
        self.nodes
            .iter()
            .flat_map(|(_, node)| node.iter())
            .filter(|node| node.distance_from_entry == Some(0))
    }

    pub(crate) fn instances(&self, plugin_id: PluginTypeId) -> impl Iterator<Item = &PluginNode> {
        self.nodes
            .get(&plugin_id)
            .into_iter()
            .flat_map(|instances| instances.iter())
    }

    pub(crate) fn insert_edge(&mut self, from: RegistrationId, to: PluginTypeId) {
        self.dependency_edges.entry(from).or_default().push(to);
    }
}

/// The final order for things to be registered in.
#[allow(unused)]
pub(crate) struct OrderedPluginItems(Vec<DeclrItem>);

#[allow(unused)]
pub(crate) struct ItemsGraph {}

/// Items that can be added to a world.
#[allow(unused)]
pub(crate) enum DeclrItem {
    Message(MessageRegistration),
    // etc.
}
