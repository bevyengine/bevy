pub mod nodes;

use crate::{
    render_resource::{RenderResource, ResourceInfo},
    renderer_2::RenderContext,
};
use legion::prelude::{Executor, Resources, Schedulable, World};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};
use uuid::Uuid;

pub enum Command {
    CopyBufferToBuffer {
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    },
}

#[derive(Default, Clone)]
pub struct CommandQueue {
    queue: Arc<Mutex<VecDeque<Command>>>,
}

impl CommandQueue {
    fn push(&mut self, command: Command) {
        self.queue.lock().unwrap().push_front(command);
    }

    pub fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    ) {
        self.push(Command::CopyBufferToBuffer {
            source_buffer,
            source_offset,
            destination_buffer,
            destination_offset,
            size,
        });
    }

    pub fn execute(&mut self, render_context: &mut dyn RenderContext) {
        for command in self.queue.lock().unwrap().drain(..) {
            match command {
                Command::CopyBufferToBuffer {
                    source_buffer,
                    source_offset,
                    destination_buffer,
                    destination_offset,
                    size,
                } => render_context.copy_buffer_to_buffer(
                    source_buffer,
                    source_offset,
                    destination_buffer,
                    destination_offset,
                    size,
                ),
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct NodeId(Uuid);

impl NodeId {
    fn new() -> Self {
        NodeId(Uuid::new_v4())
    }
}

pub struct ResourceBinding {
    pub resource: Option<RenderResource>,
    pub slot: ResourceSlot,
}

#[derive(Default)]
pub struct ResourceBindings {
    bindings: Vec<ResourceBinding>,
}

impl ResourceBindings {
    pub fn set(&mut self, index: usize, resource: RenderResource) {
        self.bindings[index].resource = Some(resource);
    }

    pub fn set_named(&mut self, name: &str, resource: RenderResource) {
        let binding = self
            .bindings
            .iter_mut()
            .find(|b| b.slot.name == name)
            .expect("Name not found");
        binding.resource = Some(resource);
    }

    pub fn get(&self, index: usize) -> Option<RenderResource> {
        self.bindings
            .get(index)
            .and_then(|binding| binding.resource)
    }

    pub fn get_named(&self, name: &str) -> Option<RenderResource> {
        self.bindings
            .iter()
            .find(|b| b.slot.name == name)
            .and_then(|binding| binding.resource)
    }
}

impl From<&ResourceSlot> for ResourceBinding {
    fn from(slot: &ResourceSlot) -> Self {
        ResourceBinding {
            resource: None,
            slot: slot.clone(),
        }
    }
}

impl From<&[ResourceSlot]> for ResourceBindings {
    fn from(slots: &[ResourceSlot]) -> Self {
        ResourceBindings {
            bindings: slots
                .iter()
                .map(|s| s.into())
                .collect::<Vec<ResourceBinding>>(),
        }
    }
}

#[derive(Clone)]
pub struct ResourceSlot {
    name: &'static str,
    resource_type: ResourceInfo,
}

impl ResourceSlot {
    pub const fn new(name: &'static str, resource_type: ResourceInfo) -> Self {
        ResourceSlot {
            name,
            resource_type,
        }
    }
}

pub trait Node: Send + Sync + 'static {
    fn input(&self) -> &[ResourceSlot] {
        &[]
    }

    fn output(&self) -> &[ResourceSlot] {
        &[]
    }

    fn update(
        &mut self,
        world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        input: &ResourceBindings,
        output: &mut ResourceBindings,
    );
}

pub struct NodeState {
    pub node: Box<dyn Node>,
    pub input: ResourceBindings,
    pub output: ResourceBindings,
}

impl NodeState {
    pub fn new<T>(node: T) -> Self
    where
        T: Node,
    {
        NodeState {
            input: ResourceBindings::from(node.input()),
            output: ResourceBindings::from(node.output()),
            node: Box::new(node),
        }
    }
}

pub trait SystemNode: Node {
    fn get_system(&self, resources: &mut Resources) -> Box<dyn Schedulable>;
}

#[derive(Default)]
pub struct RenderGraph2 {
    nodes: HashMap<NodeId, NodeState>,
    new_systems: Vec<Box<dyn Schedulable>>,
    system_executor: Option<Executor>,
}

impl RenderGraph2 {
    pub fn add_node<T>(&mut self, node: T) -> NodeId
    where
        T: Node + 'static,
    {
        let id = NodeId::new();
        self.nodes.insert(id, NodeState::new(node));
        id
    }

    pub fn add_system_node<T>(&mut self, node: T, resources: &mut Resources) -> NodeId
    where
        T: SystemNode + 'static,
    {
        let id = NodeId::new();
        self.new_systems.push(node.get_system(resources));
        self.nodes.insert(id, NodeState::new(node));
        id
    }

    pub fn take_executor(&mut self) -> Option<Executor> {
        // rebuild executor if there are new systems
        if self.new_systems.len() > 0 {
            let mut systems = self
                .system_executor
                .take()
                .map(|executor| executor.into_vec())
                .unwrap_or_else(|| Vec::new());
            for system in self.new_systems.drain(..) {
                systems.push(system);
            }

            self.system_executor = Some(Executor::new(systems));
        }

        self.system_executor.take()
    }

    pub fn set_executor(&mut self, executor: Executor) {
        self.system_executor = Some(executor);
    }
}

#[derive(Default)]
pub struct Stage<'a> {
    ordered_jobs: Vec<OrderedJob<'a>>,
}

impl<'a> Stage<'a> {
    pub fn add(&mut self, job: OrderedJob<'a>) {
        self.ordered_jobs.push(job);
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut OrderedJob<'a>> {
        self.ordered_jobs.iter_mut()
    }
}

#[derive(Default)]
pub struct OrderedJob<'a> {
    node_states: Vec<&'a mut NodeState>,
}

impl<'a> OrderedJob<'a> {
    pub fn add(&mut self, node_state: &'a mut NodeState) {
        self.node_states.push(node_state);
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut &'a mut NodeState> {
        self.node_states.iter_mut()
    }
}

pub trait RenderGraphScheduler {
    fn get_stages<'a>(&mut self, render_graph: &'a mut RenderGraph2) -> Vec<Stage<'a>>;
}

#[derive(Default)]
pub struct LinearScheduler;

impl RenderGraphScheduler for LinearScheduler {
    fn get_stages<'a>(&mut self, render_graph: &'a mut RenderGraph2) -> Vec<Stage<'a>> {
        let mut stage = Stage::default();
        let mut job = OrderedJob::default();
        for node_state in render_graph.nodes.values_mut() {
            job.add(node_state);        
        }

        stage.ordered_jobs.push(job);

        vec![stage]
    }
}