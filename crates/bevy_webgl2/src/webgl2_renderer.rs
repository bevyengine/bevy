use crate::renderer::{WebGL2RenderContext, WebGL2RenderResourceContext};
use bevy_app::prelude::*;
use bevy_ecs::{Resources, World};
use bevy_render::{
    render_graph::{
        DependentNodeStager, Edge, NodeId, RenderGraph, RenderGraphStager, ResourceSlots,
    },
    renderer::{RenderResourceContext, SharedBuffers},
};
use bevy_window::{WindowCreated, Windows};
use std::sync::Arc;

use bevy_utils::HashMap;
use parking_lot::RwLock;
use std::cell::{Ref, RefCell};

#[derive(Default)]
pub struct Device {
    context: RefCell<Option<web_sys::WebGl2RenderingContext>>,
}

impl Device {
    pub fn get_context(&self) -> std::cell::Ref<web_sys::WebGl2RenderingContext> {
        return Ref::map(self.context.borrow(), |t| {
            t.as_ref().expect("webgl context is set")
        });
    }

    pub fn set_context(&self, context: web_sys::WebGl2RenderingContext) {
        *self.context.borrow_mut() = Some(context);
    }
}

impl std::fmt::Debug for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Device: {:?}", self.context.borrow()))
    }
}

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

pub struct WebGL2Renderer {
    pub device: Arc<Device>,
    // pub window_resized_event_reader: EventReader<WindowResized>,
    pub window_created_event_reader: EventReader<WindowCreated>,
}

impl std::default::Default for WebGL2Renderer {
    fn default() -> Self {
        WebGL2Renderer {
            device: Default::default(),
            window_created_event_reader: Default::default(),
            //..Default::default()
        }
    }
}

impl WebGL2Renderer {
    pub fn handle_window_created_events(&mut self, resources: &mut Resources) {
        let events = {
            let window_created_events = resources.get::<Events<WindowCreated>>().unwrap();
            self.window_created_event_reader
                .iter(&window_created_events)
                .cloned()
                .collect::<Vec<_>>()
        };

        for window_created_event in events {
            #[cfg(feature = "bevy_winit")]
            {
                let window_id = {
                    let windows = resources.get::<Windows>().unwrap();
                    let window = windows
                        .get(window_created_event.id)
                        .expect("Received window created event for non-existent window");
                    window.id()
                };
                let render_resource_context = {
                    let winit_windows = resources.get::<bevy_winit::WinitWindows>().unwrap();
                    let winit_window = winit_windows.get_window(window_id).unwrap();
                    let device = &*resources.get::<Arc<Device>>().unwrap();
                    let mut render_resource_context =
                        WebGL2RenderResourceContext::new(device.clone());
                    render_resource_context.initialize(&winit_window);
                    render_resource_context
                };
                log::info!("window created!");
                resources.insert::<Box<dyn RenderResourceContext>>(Box::new(
                    render_resource_context.clone(),
                ));
                resources.insert(SharedBuffers::new(Box::new(render_resource_context)));
            }
        }
    }

    pub fn run_graph(&mut self, world: &mut World, resources: &mut Resources) {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        // stage nodes
        let mut stager = DependentNodeStager::loose_grouping();
        let stages = stager.get_stages(&render_graph).unwrap();
        let mut borrowed = stages.borrow(&mut render_graph);
        let render_resource_context = resources.get_mut::<Box<dyn RenderResourceContext>>();
        if render_resource_context.is_none() {
            return;
        }
        let mut render_resource_context = render_resource_context.unwrap();
        let render_resource_context = render_resource_context
            .downcast_mut::<WebGL2RenderResourceContext>()
            .unwrap();

        let node_outputs: Arc<RwLock<HashMap<NodeId, ResourceSlots>>> = Default::default();
        for stage in borrowed.iter_mut() {
            // TODO: sort jobs and slice by "amount of work" / weights
            // stage.jobs.sort_by_key(|j| j.node_states.len());

            let chunk_size = stage.jobs.len();
            for jobs_chunk in stage.jobs.chunks_mut(chunk_size) {
                let world = &*world;
                let render_resource_context = render_resource_context.clone();
                let node_outputs = node_outputs.clone();
                let mut render_context =
                    WebGL2RenderContext::new(self.device.clone(), render_resource_context);
                for job in jobs_chunk.iter_mut() {
                    for node_state in job.node_states.iter_mut() {
                        // bind inputs from connected node outputs
                        for (i, mut input_slot) in node_state.input_slots.iter_mut().enumerate() {
                            if let Edge::SlotEdge {
                                output_node,
                                output_index,
                                ..
                            } = node_state.edges.get_input_slot_edge(i).unwrap()
                            {
                                let node_outputs = node_outputs.read();
                                let outputs = if let Some(outputs) = node_outputs.get(output_node) {
                                    outputs
                                } else {
                                    panic!("node inputs not set")
                                };

                                let output_resource =
                                    outputs.get(*output_index).expect("output should be set");
                                input_slot.resource = Some(output_resource);
                            } else {
                                panic!("no edge connected to input")
                            }
                        }
                        node_state.node.update(
                            world,
                            resources,
                            &mut render_context,
                            &node_state.input_slots,
                            &mut node_state.output_slots,
                        );
                        node_outputs
                            .write()
                            .insert(node_state.id, node_state.output_slots.clone());
                    }
                }
                //sender.send(render_context.finish()).unwrap();
            }
            // })
            // .unwrap();

            // let mut command_buffers = Vec::new();
            // for _i in 0..actual_thread_count {
            //     let command_buffer = receiver.recv().unwrap();
            //     if let Some(command_buffer) = command_buffer {
            //         command_buffers.push(command_buffer);
            //     }
            // }
        }
    }

    pub fn update(&mut self, world: &mut World, resources: &mut Resources) {
        self.handle_window_created_events(resources);
        if resources.get::<Box<dyn RenderResourceContext>>().is_none() {
            return;
        }
        self.run_graph(world, resources);
        if let Some(render_resource_context) = resources.get::<Box<dyn RenderResourceContext>>() {
            render_resource_context.drop_all_swap_chain_textures();
            render_resource_context.clear_bind_groups();
        }
    }
}
