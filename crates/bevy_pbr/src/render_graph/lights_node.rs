use crate::{
    light::{AmbientLight, PointLight, PointLightUniform},
    render_graph::uniform,
};
use arrayvec::ArrayVec;
use bevy_ecs::{
    system::{BoxedSystem, IntoSystem, Local, Query, Res, ResMut},
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
use crevice::std140::{self, AsStd140, StaticStd140Size, Std140, WriteStd140};

use super::MAX_POINT_LIGHTS;

/// A Render Graph [Node] that write light data from the ECS to GPU buffers
#[derive(Debug, Default)]
pub struct LightsNode {
    command_queue: CommandQueue,
    max_point_lights: usize,
}

impl LightsNode {
    pub fn new(max_lights: usize) -> Self {
        LightsNode {
            max_point_lights: max_lights,
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

impl SystemNode for LightsNode {
    fn get_system(&self) -> BoxedSystem {
        let system = lights_node_system.system().config(|config| {
            config.0 = Some(LightsNodeSystemState {
                command_queue: self.command_queue.clone(),
                max_point_lights: self.max_point_lights,
                light_buffer: None,
                staging_buffer: None,
            })
        });
        Box::new(system)
    }
}

struct LightsUniform<'a> {
    ambient_light: &'a AmbientLight,
    point_light_count: u32,
    point_lights: ArrayVec<PointLightUniform, MAX_POINT_LIGHTS>,
}

impl<'a> WriteStd140 for LightsUniform<'a> {
    fn write_std140<W: std::io::Write>(
        &self,
        writer: &mut std140::Writer<W>,
    ) -> std::io::Result<usize> {
        let offset = writer.write(self.ambient_light)?;
        writer.write(&self.point_light_count)?;
        writer.write(self.point_lights.as_slice())?;
        Ok(offset)
    }
}

impl<'a> StaticStd140Size for LightsUniform<'a> {
    fn std140_size_static() -> usize {
        let mut offset = 0;
        offset += crevice::internal::align_offset(
            offset,
            <AmbientLight as AsStd140>::Std140Type::ALIGNMENT,
        ) + std::mem::size_of::<<AmbientLight as AsStd140>::Std140Type>();
        dbg!(&offset);
        offset += crevice::internal::align_offset(offset, <u32 as AsStd140>::Std140Type::ALIGNMENT)
            + std::mem::size_of::<<u32 as AsStd140>::Std140Type>();
        dbg!(&offset);
        offset += MAX_POINT_LIGHTS
            * (crevice::internal::align_offset(
                offset,
                <PointLightUniform as AsStd140>::Std140Type::ALIGNMENT,
            ) + std::mem::size_of::<<PointLightUniform as AsStd140>::Std140Type>());
        dbg!(&offset);
        offset
    }
}

/// Local "lights node system" state
#[derive(Debug, Default)]
pub struct LightsNodeSystemState {
    light_buffer: Option<BufferId>,
    staging_buffer: Option<BufferId>,
    command_queue: CommandQueue,
    max_point_lights: usize,
}

pub fn lights_node_system(
    mut state: Local<LightsNodeSystemState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    ambient_light_resource: Res<AmbientLight>,
    // TODO: this write on RenderResourceBindings will prevent this system from running in parallel
    // with other systems that do the same
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    query: Query<(&PointLight, &GlobalTransform)>,
) {
    let state = &mut state;
    let render_resource_context = &**render_resource_context;

    let point_light_count = query.iter().count() as u32;
    let lights_uniform = LightsUniform {
        ambient_light: &ambient_light_resource,
        point_light_count,
        point_lights: query
            .iter()
            .map(PointLightUniform::from_tuple)
            .take(MAX_POINT_LIGHTS)
            .collect(),
    };

    if let Some(staging_buffer) = state.staging_buffer {
        if point_light_count == 0 {
            return;
        }

        render_resource_context.map_buffer(staging_buffer, BufferMapMode::Write);
    } else {
        let buffer = render_resource_context.create_buffer(BufferInfo {
            size: LightsUniform::std140_size_static(),
            buffer_usage: BufferUsage::UNIFORM | BufferUsage::COPY_SRC | BufferUsage::COPY_DST,
            ..Default::default()
        });
        render_resource_bindings.set(
            uniform::LIGHTS,
            RenderResourceBinding::Buffer {
                buffer,
                range: 0..LightsUniform::std140_size_static() as u64,
                dynamic_index: None,
            },
        );
        state.light_buffer = Some(buffer);

        let staging_buffer = render_resource_context.create_buffer(BufferInfo {
            size: LightsUniform::std140_size_static(),
            buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
            mapped_at_creation: true,
        });
        state.staging_buffer = Some(staging_buffer);
    }

    let staging_buffer = state.staging_buffer.unwrap();
    render_resource_context.write_mapped_buffer(
        staging_buffer,
        0..lights_uniform.std140_size() as u64,
        &mut |data, _renderer| {
            let mut writer = std140::Writer::new(data);

            writer
                .write(&lights_uniform)
                .expect("Failed to write lights uniform");
        },
    );
    render_resource_context.unmap_buffer(staging_buffer);
    let light_buffer = state.light_buffer.unwrap();
    state.command_queue.copy_buffer_to_buffer(
        staging_buffer,
        0,
        light_buffer,
        0,
        lights_uniform.std140_size() as u64,
    );
}
