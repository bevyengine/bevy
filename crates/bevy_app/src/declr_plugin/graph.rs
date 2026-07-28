use bevy_platform::collections::HashMap;
use core::{convert::identity, hash::Hash};
use std::{boxed::Box, collections::VecDeque, vec::Vec};

use crate::{
    plugin_data::{MessageRegistration, PluginDependency, PluginTypeId},
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
                    // If a dependency already has an existing instance
                    // registered in the graph that is suitable, we discard the
                    // plugin data that was generated from this instance.

                    // We can confidently skip adding that dependency because
                    // the semantics of plugin registration are that we're
                    // asking that a plugin exists and its config is
                    // acceptable, not that a plugin _must_ be a specific value
                    // (unless asked)

                    // This does cause a bit of a problem when it comes to
                    // thinking about how plugins become detached from the
                    // dependencies they do register.
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
                        // There already exists a suitable version of this
                        // plugin in the graph, so queue up a version of this
                        // dependency that doesn't contain any info other than
                        // the fact that a dependency to that plugin does exist.
                        dependency_queue.push_back((
                            reg_id,
                            PluginDependency {
                                data: None,
                                type_id: dependency.type_id,
                            },
                        ));
                        continue;
                    }
                    dependency_queue.push_back((reg_id, dependency));
                }
                if can_zst_optimize {
                    zst_already_expanded.insert(plugin_id, reg_id);
                }
            } else if dependency.data.is_none() && graph.instances(dependency.type_id).count() == 0
            {
            } else {
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

    pub(crate) fn can_build(&self) -> bool {
        // We can't build the next graph unless each plugin that is a
        // dependency has a valid candidate.
        self.nodes
            .iter()
            .filter(|(plugin_id, _)| self.is_depended_on(**plugin_id))
            .all(|(plugin_id, _)| self.candidate_exists(*plugin_id).is_ok())
    }

    pub(crate) fn is_depended_on(&self, plugin_id: PluginTypeId) -> bool {
        self.dependency_edges
            .values()
            .any(|ids| ids.contains(&plugin_id))
    }

    pub(crate) fn candidate_exists(&self, plugin_id: PluginTypeId) -> Result<RegistrationId, ()> {
        let mut working = None;
        'outer: for candidate_instance in self.instances(plugin_id) {
            let candidate = candidate_instance.registration_id;
            for depends_on in self.has_approvals_for_plugin(plugin_id) {
                if let Some(approval) = depends_on.output.plugin_approval.get(&plugin_id)
                    && approval.approves(&candidate_instance.plugin_data)
                {
                    continue;
                } else {
                    // abandon current candidate, and move onto the next one.
                    continue 'outer;
                }
            }
            match &working.and_then(|working| self.get(working)) {
                Some(prev_candidate) => {
                    // Two cases where we write a new candidate when we already
                    // have one:
                    //
                    // 1. Previous instance wasn't an entry point, but this
                    // instance was.
                    // 2. Both previous instance and current instance were
                    // entry points, but the current entry point has a higher
                    // registration id.
                    //
                    // This does mean we have a preference for later entry
                    // points, over earlier ones. This could cause some
                    // unexpected behavior and maybe it's better to just
                    // highlight to the user what's gone on and prevent that.
                    if candidate_instance.output.is_entry_point
                        && (!prev_candidate.output.is_entry_point
                            || candidate_instance.registration_id > prev_candidate.registration_id)
                    {
                        working = Some(candidate_instance.registration_id)
                    }
                }
                None => {
                    working = Some(candidate);
                }
            }
        }
        working.ok_or(())
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
