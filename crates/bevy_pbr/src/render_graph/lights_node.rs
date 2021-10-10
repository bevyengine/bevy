use crate::{
    light::{
        AmbientLight, DirectionalLight, DirectionalLightUniform, PointLight, PointLightUniform,
    },
    render_graph::uniform,
};
use bevy_core::{bytes_of, Pod, Zeroable};
use bevy_ecs::{
    system::{BoxedSystem, ConfigurableSystem, Local, Query, Res, ResMut},
    world::World,
};
use bevy_render::{
    render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
    renderer::{
        BufferId, BufferInfo, BufferMapMode, BufferUsage, RenderContext, RenderResourceBinding,
        RenderResourceBindings, RenderResourceContext,
    },
};
use bevy_transform::prelude::*;

/// A Render Graph [Node] that write light data from the ECS to GPU buffers
#[derive(Debug, Default)]
pub struct LightsNode {
    command_queue: CommandQueue,
    max_point_lights: usize,
    max_dir_lights: usize,
}

impl LightsNode {
    pub fn new(max_point_lights: usize, max_dir_lights: usize) -> Self {
        LightsNode {
            max_point_lights,
            max_dir_lights,
            command_queue: CommandQueue::default(),
        }
    }
}

impl Node for LightsNode {
    fn update(
        &mut self,
        _world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        self.command_queue.execute(render_context);
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct LightCount {
    // storing as a `[u32; 4]` for memory alignement
    // Index 0 is for point lights,
    // Index 1 is for directional lights
    pub num_lights: [u32; 4],
}

impl SystemNode for LightsNode {
    fn get_system(&self) -> BoxedSystem {
        let system = lights_node_system.config(|config| {
            config.0 = Some(LightsNodeSystemState {
                command_queue: self.command_queue.clone(),
                max_point_lights: self.max_point_lights,
                max_dir_lights: self.max_dir_lights,
                light_buffer: None,
                staging_buffer: None,
            })
        });
        Box::new(system)
    }
}

/// Local "lights node system" state
#[derive(Debug, Default)]
pub struct LightsNodeSystemState {
    light_buffer: Option<BufferId>,
    staging_buffer: Option<BufferId>,
    command_queue: CommandQueue,
    max_point_lights: usize,
    max_dir_lights: usize,
}

pub fn lights_node_system(
    mut state: Local<LightsNodeSystemState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    ambient_light_resource: Res<AmbientLight>,
    // TODO: this write on RenderResourceBindings will prevent this system from running in parallel
    // with other systems that do the same
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    point_lights: Query<(&PointLight, &GlobalTransform)>,
    dir_lights: Query<&DirectionalLight>,
) {
    let state = &mut state;
    let render_resource_context = &**render_resource_context;

    // premultiply ambient brightness
    let ambient_light: [f32; 4] =
        (ambient_light_resource.color * ambient_light_resource.brightness).into();
    let ambient_light_size = std::mem::size_of::<[f32; 4]>();

    let point_light_count = point_lights.iter().len().min(state.max_point_lights);
    let point_light_size = std::mem::size_of::<PointLightUniform>();
    let point_light_array_size = point_light_size * point_light_count;
    let point_light_array_max_size = point_light_size * state.max_point_lights;

    let dir_light_count = dir_lights.iter().len().min(state.max_dir_lights);
    let dir_light_size = std::mem::size_of::<DirectionalLightUniform>();
    let dir_light_array_size = dir_light_size * dir_light_count;
    let dir_light_array_max_size = dir_light_size * state.max_dir_lights;

    let light_count_size = ambient_light_size + std::mem::size_of::<LightCount>();

    let point_light_uniform_start = light_count_size;
    let point_light_uniform_end = light_count_size + point_light_array_size;

    let dir_light_uniform_start = light_count_size + point_light_array_max_size;
    let dir_light_uniform_end =
        light_count_size + point_light_array_max_size + dir_light_array_size;

    let max_light_uniform_size =
        light_count_size + point_light_array_max_size + dir_light_array_max_size;

    if let Some(staging_buffer) = state.staging_buffer {
        if point_light_count == 0 && dir_light_count == 0 {
            return;
        }

        render_resource_context.map_buffer(staging_buffer, BufferMapMode::Write);
    } else {
        let buffer = render_resource_context.create_buffer(BufferInfo {
            size: max_light_uniform_size,
            buffer_usage: BufferUsage::UNIFORM | BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
            ..Default::default()
        });
        render_resource_bindings.set(
            uniform::LIGHTS,
            RenderResourceBinding::Buffer {
                buffer,
                range: 0..max_light_uniform_size as u64,
                dynamic_index: None,
            },
        );
        state.light_buffer = Some(buffer);

        let staging_buffer = render_resource_context.create_buffer(BufferInfo {
            size: max_light_uniform_size,
            buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
            mapped_at_creation: true,
        });
        state.staging_buffer = Some(staging_buffer);
    }

    let staging_buffer = state.staging_buffer.unwrap();
    render_resource_context.write_mapped_buffer(
        staging_buffer,
        0..max_light_uniform_size as u64,
        &mut |data, _renderer| {
            // ambient light
            data[0..ambient_light_size].copy_from_slice(bytes_of(&ambient_light));

            // light count
            data[ambient_light_size..light_count_size].copy_from_slice(bytes_of(&[
                point_light_count as u32,
                dir_light_count as u32,
                0,
                0,
            ]));

            // point light array
            for ((point_light, global_transform), slot) in point_lights.iter().zip(
                data[point_light_uniform_start..point_light_uniform_end]
                    .chunks_exact_mut(point_light_size),
            ) {
                slot.copy_from_slice(bytes_of(&PointLightUniform::new(
                    point_light,
                    global_transform,
                )));
            }

            // directional light array
            for (dir_light, slot) in dir_lights.iter().zip(
                data[dir_light_uniform_start..dir_light_uniform_end]
                    .chunks_exact_mut(dir_light_size),
            ) {
                slot.copy_from_slice(bytes_of(&DirectionalLightUniform::new(dir_light)));
            }
        },
    );
    render_resource_context.unmap_buffer(staging_buffer);
    let light_buffer = state.light_buffer.unwrap();
    state.command_queue.copy_buffer_to_buffer(
        staging_buffer,
        0,
        light_buffer,
        0,
        max_light_uniform_size as u64,
    );
}
