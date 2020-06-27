use bevy_render::{
    render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
    render_resource::{BufferInfo, BufferUsage, RenderResourceBinding, RenderResourceBindings},
    renderer::{RenderContext, RenderResourceContext},
};

use crate::{
    light::{Light, LightRaw},
    uniform,
};
use bevy_core::bytes::{AsBytes, Byteable};
use bevy_transform::prelude::*;
use legion::prelude::*;

#[derive(Default)]
pub struct LightsNode {
    command_queue: CommandQueue,
    max_lights: usize,
}

impl LightsNode {
    pub fn new(max_lights: usize) -> Self {
        LightsNode {
            max_lights,
            command_queue: CommandQueue::default(),
        }
    }
}

impl Node for LightsNode {
    fn update(
        &mut self,
        _world: &World,
        _resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        self.command_queue.execute(render_context);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LightCount {
    pub num_lights: [u32; 4],
}

unsafe impl Byteable for LightCount {}

impl SystemNode for LightsNode {
    fn get_system(&self) -> Box<dyn Schedulable> {
        let mut light_buffer = None;
        let mut lights_are_dirty = true;
        // TODO: merge these
        let mut tmp_count_buffer = None;
        let mut tmp_light_buffer = None;
        let mut command_queue = self.command_queue.clone();
        let max_lights = self.max_lights;
        (move |render_resource_context: Res<Box<dyn RenderResourceContext>>,
               // TODO: this write on RenderResourceAssignments will prevent this system from running in parallel with other systems that do the same
               mut render_resource_bindings: ResMut<RenderResourceBindings>,
               world: &mut SubWorld,
               query: &mut Query<(Read<Light>, Read<Transform>, Read<Translation>)>| {
            if !lights_are_dirty {
                return;
            }

            let render_resource_context = &**render_resource_context;
            if light_buffer.is_none() {
                let light_uniform_size = std::mem::size_of::<LightCount>()
                    + max_lights * std::mem::size_of::<LightRaw>();

                let buffer = render_resource_context.create_buffer(BufferInfo {
                    size: light_uniform_size,
                    buffer_usage: BufferUsage::UNIFORM
                        | BufferUsage::COPY_SRC
                        | BufferUsage::COPY_DST,
                    ..Default::default()
                });
                render_resource_bindings.set(
                    uniform::LIGHTS,
                    RenderResourceBinding::Buffer {
                        buffer,
                        range: 0..light_uniform_size as u64,
                        dynamic_index: None,
                    },
                );
                light_buffer = Some(buffer);
            }

            let light_count = query.iter(world).count();

            if light_count == 0 {
                return;
            }

            lights_are_dirty = false;
            let size = std::mem::size_of::<LightRaw>();
            let total_size = size * light_count;
            let light_count_size = std::mem::size_of::<LightCount>();

            if let Some(old_tmp_light_buffer) = tmp_light_buffer {
                render_resource_context.remove_buffer(old_tmp_light_buffer);
            }

            if let Some(old_tmp_count_buffer) = tmp_count_buffer {
                render_resource_context.remove_buffer(old_tmp_count_buffer);
            }

            tmp_light_buffer = Some(render_resource_context.create_buffer_mapped(
                BufferInfo {
                    size: total_size,
                    buffer_usage: BufferUsage::COPY_SRC,
                    ..Default::default()
                },
                &mut |data, _renderer| {
                    for ((light, transform, translation), slot) in
                        query.iter(world).zip(data.chunks_exact_mut(size))
                    {
                        slot.copy_from_slice(
                            LightRaw::from(&light, &transform.value, &translation).as_bytes(),
                        );
                    }
                },
            ));
            tmp_count_buffer = Some(render_resource_context.create_buffer_mapped(
                BufferInfo {
                    size: light_count_size,
                    buffer_usage: BufferUsage::COPY_SRC,
                    ..Default::default()
                },
                &mut |data, _renderer| {
                    data.copy_from_slice([light_count as u32, 0, 0, 0].as_bytes());
                },
            ));

            command_queue.copy_buffer_to_buffer(
                tmp_count_buffer.unwrap(),
                0,
                light_buffer.unwrap(),
                0,
                light_count_size as u64,
            );

            command_queue.copy_buffer_to_buffer(
                tmp_light_buffer.unwrap(),
                0,
                light_buffer.unwrap(),
                light_count_size as u64,
                total_size as u64,
            );
        })
        .system()
    }
}
