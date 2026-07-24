use bevy_platform::collections::HashMap;
use core::hash::Hash;
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
        let mut dependency_stack = VecDeque::new();
        for (item, output) in self.nodes {
            if output.is_zero_sized_optimizable
                && !zst_already_expanded.contains_key(&output.working_plugin)
            {
                let type_id = output.working_plugin;
                let (reg_id, dependencies) = graph.insert_node(output.working_plugin, item, output);
                dependency_stack.extend(dependencies.0.into_iter().map(|d| (reg_id, d)));
                zst_already_expanded.insert(type_id, reg_id);
            } else if !output.is_zero_sized_optimizable {
            }
        }
        // TODO: detect cycles in expansion + stop adding when "expanded enough" + solved.
        // mean moving this logic to the PluginRegistrationGraph building side.
        while let Some((from, dependency)) = dependency_stack.pop_front() {
            if !zst_already_expanded.contains_key(&dependency.type_id)
                && let Some((dyn_plugin, output_fn)) = dependency.data
            {
                let Some(output) = output_fn(dyn_plugin.as_ref()) else {
                    continue;
                };
                let plugin_id = output.working_plugin;
                let can_zst_optimize = output.is_zero_sized_optimizable;
                let (reg_id, dependencies) = graph.insert_node(plugin_id, dyn_plugin, output);
                graph.insert_edge(from, reg_id);
                dependency_stack.extend(dependencies.0.into_iter().map(|d| (reg_id, d)));
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
pub(crate) struct RegistrationId(usize);

pub(crate) struct PluginRegistrationGraph {
    registration_counter: usize,
    nodes:
        HashMap<PluginTypeId, Vec<(RegistrationId, Box<dyn DeclarativePlugin>, PluginOutput<()>)>>,
    registration_type_association: HashMap<RegistrationId, PluginTypeId>,
    dependency_edges: HashMap<RegistrationId, Vec<RegistrationId>>,
}

impl PluginRegistrationGraph {
    fn new_id(&mut self) -> RegistrationId {
        let id = RegistrationId(self.registration_counter);
        self.registration_counter += 1;
        id
    }

    pub(crate) fn new() -> Self {
        Self {
            registration_counter: 0,
            nodes: HashMap::new(),
            dependency_edges: HashMap::new(),
            registration_type_association: HashMap::new(),
        }
    }

    #[must_use]
    pub(crate) fn insert_node<D>(
        &mut self,
        id: PluginTypeId,
        plugin_data: Box<dyn DeclarativePlugin>,
        output: PluginOutput<D>,
    ) -> (RegistrationId, D) {
        let registration_id = self.new_id();
        let (erased_output, dependencies) = output.extract_dependencies();
        self.nodes
            .entry(id)
            .or_default()
            .push((registration_id, plugin_data, erased_output));
        self.registration_type_association
            .insert(registration_id, id);
        (registration_id, dependencies)
    }

    pub(crate) fn insert_edge(&mut self, from: RegistrationId, to: RegistrationId) {
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
