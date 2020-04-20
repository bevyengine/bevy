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

pub struct ResourceSlot {
    name: Option<String>,
    resource_type: ResourceInfo,
}

pub struct ResourceSlotBinding {
    resource: RenderResource,
}

pub struct NodeDescriptor {
    pub inputs: Vec<ResourceSlot>,
    pub outputs: Vec<ResourceSlot>,
}

pub trait Node: Send + Sync + 'static {
    fn descriptor(&self) -> &NodeDescriptor;
    fn update(
        &mut self,
        world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
    );
}

pub trait SystemNode: Node {
    fn get_system(&self, resources: &mut Resources) -> Box<dyn Schedulable>;
}

#[derive(Default)]
pub struct RenderGraph2 {
    nodes: HashMap<NodeId, Box<dyn Node>>,
    new_systems: Vec<Box<dyn Schedulable>>,
    system_executor: Option<Executor>,
}

impl RenderGraph2 {
    pub fn add_node<T>(&mut self, node: T) -> NodeId
    where
        T: Node + 'static,
    {
        let id = NodeId::new();
        self.nodes.insert(id, Box::new(node));
        id
    }

    pub fn add_system_node<T>(&mut self, node: T, resources: &mut Resources) -> NodeId
    where
        T: SystemNode + 'static,
    {
        let id = NodeId::new();
        self.new_systems.push(node.get_system(resources));
        self.nodes.insert(id, Box::new(node));
        id
    }

    pub fn get_schedule(&mut self) -> impl Iterator<Item = &mut Box<dyn Node>> {
        self.nodes.values_mut()
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
