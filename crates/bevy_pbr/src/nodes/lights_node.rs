use bevy_render::{
    render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
    render_resource::{
        BufferInfo, BufferUsage, RenderResourceAssignment, RenderResourceAssignments,
    },
    renderer::{RenderContext, RenderResources},
};

use crate::{
    light::{Light, LightRaw},
    uniform,
};
use bevy_transform::prelude::*;
use legion::prelude::*;
use zerocopy::AsBytes;

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
#[derive(Clone, Copy, AsBytes)]
pub struct LightCount {
    pub num_lights: [u32; 4],
}

impl SystemNode for LightsNode {
    fn get_system(&self) -> Box<dyn Schedulable> {
        let mut light_buffer = None;
        let mut lights_are_dirty = true;
        // TODO: merge these
        let mut tmp_count_buffer = None;
        let mut tmp_light_buffer = None;
        let mut command_queue = self.command_queue.clone();
        let max_lights = self.max_lights;
        (move |world: &mut SubWorld,
               render_resources: Res<RenderResources>,
               // TODO: this write on RenderResourceAssignments will prevent this system from running in parallel with other systems that do the same
               mut render_resource_assignments: ResMut<RenderResourceAssignments>,
               query: &mut Query<(Read<Light>, Read<LocalToWorld>, Read<Translation>)>| {
            if !lights_are_dirty {
                return;
            }

            let render_resources = &render_resources.context;
            if light_buffer.is_none() {
                let light_uniform_size = std::mem::size_of::<LightCount>()
                    + max_lights * std::mem::size_of::<LightRaw>();

                let buffer = render_resources.create_buffer(BufferInfo {
                    size: light_uniform_size,
                    buffer_usage: BufferUsage::UNIFORM
                        | BufferUsage::COPY_SRC
                        | BufferUsage::COPY_DST,
                    ..Default::default()
                });
                render_resource_assignments.set(
                    uniform::LIGHTS,
                    RenderResourceAssignment::Buffer {
                        resource: buffer,
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
                render_resources.remove_buffer(old_tmp_light_buffer);
            }

            if let Some(old_tmp_count_buffer) = tmp_count_buffer {
                render_resources.remove_buffer(old_tmp_count_buffer);
            }

            tmp_light_buffer = Some(render_resources.create_buffer_mapped(
                BufferInfo {
                    size: total_size,
                    buffer_usage: BufferUsage::COPY_SRC,
                    ..Default::default()
                },
                &mut |data, _renderer| {
                    for ((light, local_to_world, translation), slot) in
                        query.iter(world).zip(data.chunks_exact_mut(size))
                    {
                        slot.copy_from_slice(
                            LightRaw::from(&light, &local_to_world.0, &translation).as_bytes(),
                        );
                    }
                },
            ));
            tmp_count_buffer = Some(render_resources.create_buffer_mapped(
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
