use crate::{
    pipeline::VertexBufferDescriptors,
    render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
    render_resource::{
        BufferInfo, BufferUsage, EntitiesWaitingForAssets, RenderResource,
        RenderResourceAssignment, RenderResourceAssignments, RenderResourceAssignmentsId,
    },
    renderer::{RenderContext, RenderResourceContext, RenderResources},
    shader::{AsUniforms, FieldBindType},
    texture, Renderable,
};

use bevy_asset::{Assets, Handle};
use legion::prelude::*;
use std::{collections::HashMap, marker::PhantomData};

pub const BIND_BUFFER_ALIGNMENT: usize = 256;
#[derive(Debug)]
struct QueuedBufferWrite {
    buffer: RenderResource,
    offset: usize,
}

#[derive(Debug)]
struct BufferArrayStatus {
    new_item_count: usize,
    item_size: usize,
    aligned_size: usize,
    staging_buffer_offset: usize,
    buffer: Option<RenderResource>,
    queued_buffer_writes: Vec<QueuedBufferWrite>,
    current_item_count: usize,
    current_item_capacity: usize,
    indices: HashMap<RenderResourceAssignmentsId, usize>,
    current_index: usize,
}

impl BufferArrayStatus {
    pub fn get_or_assign_index(&mut self, id: RenderResourceAssignmentsId) -> usize {
        if let Some(offset) = self.indices.get(&id) {
            *offset
        } else {
            if self.current_index == self.current_item_capacity {
                panic!("no empty slots available in array");
            }

            let index = self.current_index;
            self.indices.insert(id, index);
            self.current_index += 1;
            index
        }
    }
}

struct UniformBufferArrays<T>
where
    T: AsUniforms,
{
    uniform_arrays: Vec<Option<(String, BufferArrayStatus)>>,
    _marker: PhantomData<T>,
}

impl<T> UniformBufferArrays<T>
where
    T: AsUniforms,
{
    fn new() -> Self {
        let mut uniform_arrays = Vec::new();
        let field_infos = T::get_field_infos();
        uniform_arrays.resize_with(field_infos.len(), || None);
        UniformBufferArrays {
            uniform_arrays,
            _marker: PhantomData::default(),
        }
    }
    fn reset_new_item_counts(&mut self) {
        for buffer_status in self.uniform_arrays.iter_mut() {
            if let Some((_name, buffer_status)) = buffer_status {
                buffer_status.new_item_count = 0;
            }
        }
    }

    fn increment_uniform_counts(&mut self, uniforms: &T) {
        for (i, field_info) in T::get_field_infos().iter().enumerate() {
            if let Some(FieldBindType::Uniform { size }) | Some(FieldBindType::Buffer { size }) =
                uniforms.get_field_bind_type(&field_info.name)
            {
                if let Some((ref _name, ref mut buffer_array_status)) = self.uniform_arrays[i] {
                    buffer_array_status.new_item_count += 1;
                } else {
                    self.uniform_arrays[i] = Some((
                        field_info.uniform_name.to_string(),
                        BufferArrayStatus {
                            new_item_count: 1,
                            queued_buffer_writes: Vec::new(),
                            aligned_size: Self::get_aligned_dynamic_uniform_size(size),
                            item_size: size,
                            staging_buffer_offset: 0,
                            buffer: None,
                            current_index: 0,
                            current_item_count: 0,
                            current_item_capacity: 0,
                            indices: HashMap::new(),
                        },
                    ))
                }
            }
        }
    }

    fn get_aligned_dynamic_uniform_size(data_size: usize) -> usize {
        BIND_BUFFER_ALIGNMENT * ((data_size as f32 / BIND_BUFFER_ALIGNMENT as f32).ceil() as usize)
    }

    fn setup_buffer_arrays(
        &mut self,
        render_resource_context: &dyn RenderResourceContext,
        dynamic_uniforms: bool,
    ) {
        for buffer_array_status in self.uniform_arrays.iter_mut() {
            if let Some((_name, buffer_array_status)) = buffer_array_status {
                if dynamic_uniforms {
                    Self::setup_buffer_array(buffer_array_status, render_resource_context, true);
                }

                buffer_array_status.queued_buffer_writes =
                    Vec::with_capacity(buffer_array_status.new_item_count);
            }
        }
    }

    fn setup_buffer_array(
        buffer_array_status: &mut BufferArrayStatus,
        render_resource_context: &dyn RenderResourceContext,
        align: bool,
    ) {
        if buffer_array_status.current_item_capacity < buffer_array_status.new_item_count {
            let new_capacity =
                buffer_array_status.new_item_count + buffer_array_status.new_item_count / 2;
            let mut item_size = buffer_array_status.item_size;
            if align {
                item_size = Self::get_aligned_dynamic_uniform_size(item_size);
            }

            let total_size = item_size * new_capacity;

            let buffer = render_resource_context.create_buffer(BufferInfo {
                size: total_size,
                buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
            });

            buffer_array_status.current_item_capacity = new_capacity;

            log::trace!(
                "creating buffer for uniform {}. size: {} item_capacity: {} item_size: {}",
                std::any::type_name::<T>(),
                total_size,
                new_capacity,
                item_size
            );

            buffer_array_status.buffer = Some(buffer);
        }
    }
    fn update_staging_buffer_offsets(&mut self) -> usize {
        let mut size = 0;
        for dynamic_buffer_array_status in self.uniform_arrays.iter_mut() {
            if let Some((_name, ref mut buffer_array_status)) = dynamic_buffer_array_status {
                buffer_array_status.staging_buffer_offset = size;
                size += buffer_array_status.item_size * buffer_array_status.new_item_count;
            }
        }

        size
    }

    fn setup_uniform_buffer_resources(
        &mut self,
        uniforms: &T,
        dynamic_uniforms: bool,
        render_resources: &dyn RenderResourceContext,
        render_resource_assignments: &mut RenderResourceAssignments,
        staging_buffer: &mut [u8],
    ) {
        for (i, field_info) in T::get_field_infos().iter().enumerate() {
            let bind_type = uniforms.get_field_bind_type(&field_info.name);
            match bind_type {
                Some(FieldBindType::Uniform { size }) | Some(FieldBindType::Buffer { size }) => {
                    let (_name, uniform_buffer_status) = self.uniform_arrays[i].as_mut().unwrap();
                    let range = 0..size as u64;
                    let (target_buffer, target_offset) = if dynamic_uniforms {
                        let buffer = uniform_buffer_status.buffer.unwrap();
                        let index = uniform_buffer_status
                            .get_or_assign_index(render_resource_assignments.id);
                        render_resource_assignments.set(
                            &field_info.uniform_name,
                            RenderResourceAssignment::Buffer {
                                resource: buffer,
                                dynamic_index: Some(
                                    (index * uniform_buffer_status.aligned_size) as u32,
                                ),
                                range,
                            },
                        );
                        (buffer, index * uniform_buffer_status.aligned_size)
                    } else {
                        let resource = match render_resource_assignments
                            .get(field_info.uniform_name)
                        {
                            Some(assignment) => assignment.get_resource(),
                            None => {
                                let usage = if let Some(FieldBindType::Buffer { .. }) = bind_type {
                                    BufferUsage::STORAGE
                                } else {
                                    BufferUsage::UNIFORM
                                };
                                let resource = render_resources.create_buffer(BufferInfo {
                                    size,
                                    buffer_usage: BufferUsage::COPY_DST | usage,
                                    ..Default::default()
                                });

                                render_resource_assignments.set(
                                    &field_info.uniform_name,
                                    RenderResourceAssignment::Buffer {
                                        resource,
                                        range,
                                        dynamic_index: None,
                                    },
                                );
                                resource
                            }
                        };

                        (resource, 0)
                    };

                    let staging_buffer_start = uniform_buffer_status.staging_buffer_offset
                        + (uniform_buffer_status.queued_buffer_writes.len()
                            * uniform_buffer_status.item_size);
                    let uniform_byte_len = uniforms.uniform_byte_len(&field_info.uniform_name);
                    if uniform_byte_len > 0 {
                        if size != uniform_byte_len {
                            panic!("The number of bytes produced for {} do not match the expected count. Actual: {}. Expected: {}.", field_info.uniform_name, uniform_byte_len, size);
                        }

                        uniforms.write_uniform_bytes(
                            &field_info.uniform_name,
                            &mut staging_buffer
                                [staging_buffer_start..(staging_buffer_start + uniform_byte_len)],
                        );
                    } else {
                        panic!(
                            "failed to get data from uniform: {}",
                            field_info.uniform_name
                        );
                    };

                    uniform_buffer_status
                        .queued_buffer_writes
                        .push(QueuedBufferWrite {
                            buffer: target_buffer,
                            offset: target_offset,
                        });
                }
                Some(FieldBindType::Texture) => { /* ignore textures */ }
                None => { /* ignore None */ }
            }
        }
    }

    fn copy_staging_buffer_to_final_buffers(
        &mut self,
        command_queue: &mut CommandQueue,
        staging_buffer: RenderResource,
    ) {
        for uniform_buffer_status in self.uniform_arrays.iter_mut() {
            if let Some((_name, buffer_array_status)) = uniform_buffer_status {
                let start = buffer_array_status.staging_buffer_offset;
                for (i, queued_buffer_write) in buffer_array_status
                    .queued_buffer_writes
                    .drain(..)
                    .enumerate()
                {
                    command_queue.copy_buffer_to_buffer(
                        staging_buffer,
                        (start + (i * buffer_array_status.item_size)) as u64,
                        queued_buffer_write.buffer,
                        queued_buffer_write.offset as u64,
                        buffer_array_status.item_size as u64,
                    )
                }
            }
        }
    }
}

// TODO: use something like this to remove redundancy between AssetUniformNode and UniformNode
// fn update_uniforms<T>(
//     render_resource_context: &dyn RenderResourceContext,
//     staging_buffer_resource: &mut Option<RenderResource>,
//     uniform_buffer_arrays: &mut UniformBufferArrays<T>,
//     command_queue: &mut CommandQueue,
//     dynamic_uniforms: bool,
//     increment_uniform_counts: impl Fn(),
//     update_textures: impl Fn(),
//     update_uniform_buffers: impl Fn(&mut [u8]),
// ) where
//     T: AsUniforms,
// {
//     if let Some(staging_buffer_resource) = staging_buffer_resource {
//         render_resource_context.remove_buffer(*staging_buffer_resource);
//     }
//     *staging_buffer_resource = None;

//     uniform_buffer_arrays.reset_new_item_counts();

//     increment_uniform_counts();

//     uniform_buffer_arrays.setup_buffer_arrays(render_resource_context, dynamic_uniforms);
//     let staging_buffer_size = uniform_buffer_arrays.update_staging_buffer_offsets();

//     update_textures();

//     if staging_buffer_size == 0 {
//         let mut staging_buffer: [u8; 0] = [];
//         update_uniform_buffers(&mut staging_buffer);
//     } else {
//         let staging_buffer = render_resource_context.create_buffer_mapped(
//             BufferInfo {
//                 buffer_usage: BufferUsage::COPY_SRC,
//                 size: staging_buffer_size,
//                 ..Default::default()
//             },
//             &mut |staging_buffer, _render_resources| {
//                 update_uniform_buffers(staging_buffer);
//             },
//         );

//         uniform_buffer_arrays
//             .copy_staging_buffer_to_final_buffers(command_queue, staging_buffer);

//         *staging_buffer_resource = Some(staging_buffer);
//     }
// }

#[derive(Default)]
pub struct UniformNode<T>
where
    T: AsUniforms,
{
    command_queue: CommandQueue,
    dynamic_uniforms: bool,
    _marker: PhantomData<T>,
}

impl<T> UniformNode<T>
where
    T: AsUniforms,
{
    pub fn new(dynamic_uniforms: bool) -> Self {
        UniformNode {
            command_queue: CommandQueue::default(),
            dynamic_uniforms,
            _marker: PhantomData::default(),
        }
    }
}

impl<T> Node for UniformNode<T>
where
    T: AsUniforms,
{
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

impl<T> SystemNode for UniformNode<T>
where
    T: AsUniforms,
{
    fn get_system(&self) -> Box<dyn Schedulable> {
        let mut command_queue = self.command_queue.clone();
        let mut uniform_buffer_arrays = UniformBufferArrays::<T>::new();
        let dynamic_uniforms = self.dynamic_uniforms;
        // TODO: maybe run "update" here
        SystemBuilder::new(format!(
            "uniform_resource_provider::<{}>",
            std::any::type_name::<T>()
        ))
        .read_resource::<RenderResources>()
        .read_resource::<EntitiesWaitingForAssets>()
        // TODO: this write on RenderResourceAssignments will prevent this system from running in parallel with other systems that do the same
        .with_query(<(Read<T>, Read<Renderable>)>::query())
        .with_query(<(Read<T>, Write<Renderable>)>::query())
        .build(
            move |_,
                  world,
                  (render_resources, entities_waiting_for_assets),
                  (read_uniform_query, write_uniform_query)| {
                let render_resource_context = &*render_resources.context;

                uniform_buffer_arrays.reset_new_item_counts();
                // update uniforms info
                for (uniforms, renderable) in read_uniform_query.iter(world) {
                    if !renderable.is_visible {
                        return;
                    }

                    if renderable.is_instanced {
                        panic!("instancing not currently supported");
                    } else {
                        uniform_buffer_arrays.increment_uniform_counts(&uniforms);
                    }
                }

                uniform_buffer_arrays
                    .setup_buffer_arrays(render_resource_context, dynamic_uniforms);
                let staging_buffer_size = uniform_buffer_arrays.update_staging_buffer_offsets();

                for (entity, (uniforms, mut renderable)) in
                    write_uniform_query.iter_entities_mut(world)
                {
                    if !renderable.is_visible {
                        return;
                    }

                    if renderable.is_instanced {
                        panic!("instancing not currently supported");
                    } else {
                        setup_uniform_texture_resources::<T>(
                            entity,
                            &uniforms,
                            render_resource_context,
                            entities_waiting_for_assets,
                            &mut renderable.render_resource_assignments,
                        )
                    }
                }

                if staging_buffer_size == 0 {
                    let mut staging_buffer: [u8; 0] = [];
                    for (uniforms, mut renderable) in write_uniform_query.iter_mut(world) {
                        if !renderable.is_visible {
                            return;
                        }

                        if renderable.is_instanced {
                            panic!("instancing not currently supported");
                        } else {
                            uniform_buffer_arrays.setup_uniform_buffer_resources(
                                &uniforms,
                                dynamic_uniforms,
                                render_resource_context,
                                &mut renderable.render_resource_assignments,
                                &mut staging_buffer,
                            );
                        }
                    }
                } else {
                    let staging_buffer = render_resource_context.create_buffer_mapped(
                        BufferInfo {
                            buffer_usage: BufferUsage::COPY_SRC,
                            size: staging_buffer_size,
                            ..Default::default()
                        },
                        &mut |mut staging_buffer, _render_resources| {
                            for (uniforms, mut renderable) in write_uniform_query.iter_mut(world) {
                                if !renderable.is_visible {
                                    return;
                                }

                                if renderable.is_instanced {
                                    panic!("instancing not currently supported");
                                } else {
                                    uniform_buffer_arrays.setup_uniform_buffer_resources(
                                        &uniforms,
                                        dynamic_uniforms,
                                        render_resource_context,
                                        &mut renderable.render_resource_assignments,
                                        &mut staging_buffer,
                                    );
                                }
                            }
                        },
                    );

                    uniform_buffer_arrays
                        .copy_staging_buffer_to_final_buffers(&mut command_queue, staging_buffer);
                    command_queue.free_buffer(staging_buffer);
                }
            },
        )
    }
}

#[derive(Default)]
pub struct AssetUniformNode<T>
where
    T: AsUniforms,
{
    command_queue: CommandQueue,
    dynamic_uniforms: bool,
    _marker: PhantomData<T>,
}

impl<T> AssetUniformNode<T>
where
    T: AsUniforms,
{
    pub fn new(dynamic_uniforms: bool) -> Self {
        AssetUniformNode {
            command_queue: CommandQueue::default(),
            dynamic_uniforms,
            _marker: PhantomData::default(),
        }
    }
}

impl<T> Node for AssetUniformNode<T>
where
    T: AsUniforms,
{
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

impl<T> SystemNode for AssetUniformNode<T>
where
    T: AsUniforms,
{
    fn get_system(&self) -> Box<dyn Schedulable> {
        let mut command_queue = self.command_queue.clone();
        let mut uniform_buffer_arrays = UniformBufferArrays::<T>::new();
        let dynamic_uniforms = self.dynamic_uniforms;
        // TODO: maybe run "update" here
        SystemBuilder::new("uniform_resource_provider")
            .read_resource::<Assets<T>>()
            .read_resource::<RenderResources>()
            .read_resource::<EntitiesWaitingForAssets>()
            // TODO: this write on RenderResourceAssignments will prevent this system from running in parallel with other systems that do the same
            .with_query(<(Read<Handle<T>>, Read<Renderable>)>::query())
            .with_query(<(Read<Handle<T>>, Write<Renderable>)>::query())
            .build(
                move |_,
                      world,
                      (assets, render_resources, entities_waiting_for_assets),
                      (read_handle_query, write_handle_query)| {
                    let render_resource_context = &*render_resources.context;
                    uniform_buffer_arrays.reset_new_item_counts();

                    // update uniform handles info
                    for (entity, (handle, renderable)) in read_handle_query.iter_entities(world) {
                        if !renderable.is_visible {
                            return;
                        }

                        if renderable.is_instanced {
                            panic!("instancing not currently supported");
                        } else {
                            if let Some(uniforms) = assets.get(&handle) {
                                // TODO: only increment count if we haven't seen this uniform handle before
                                uniform_buffer_arrays.increment_uniform_counts(&uniforms);
                            } else {
                                entities_waiting_for_assets.add(entity)
                            }
                        }
                    }

                    uniform_buffer_arrays
                        .setup_buffer_arrays(render_resource_context, dynamic_uniforms);
                    let staging_buffer_size = uniform_buffer_arrays.update_staging_buffer_offsets();

                    for (entity, (handle, mut renderable)) in
                        write_handle_query.iter_entities_mut(world)
                    {
                        if !renderable.is_visible {
                            return;
                        }

                        if renderable.is_instanced {
                            panic!("instancing not currently supported");
                        } else {
                            if let Some(uniforms) = assets.get(&handle) {
                                setup_uniform_texture_resources::<T>(
                                    entity,
                                    &uniforms,
                                    render_resource_context,
                                    entities_waiting_for_assets,
                                    &mut renderable.render_resource_assignments,
                                )
                            }
                        }
                    }
                    if staging_buffer_size == 0 {
                        let mut staging_buffer: [u8; 0] = [];
                        for (handle, mut renderable) in write_handle_query.iter_mut(world) {
                            if !renderable.is_visible {
                                return;
                            }
                            if renderable.is_instanced {
                                panic!("instancing not currently supported");
                            } else {
                                if let Some(uniforms) = assets.get(&handle) {
                                    // TODO: only setup buffer if we haven't seen this handle before
                                    uniform_buffer_arrays.setup_uniform_buffer_resources(
                                        &uniforms,
                                        dynamic_uniforms,
                                        render_resource_context,
                                        &mut renderable.render_resource_assignments,
                                        &mut staging_buffer,
                                    );
                                }
                            }
                        }
                    } else {
                        let staging_buffer = render_resource_context.create_buffer_mapped(
                            BufferInfo {
                                buffer_usage: BufferUsage::COPY_SRC,
                                size: staging_buffer_size,
                                ..Default::default()
                            },
                            &mut |mut staging_buffer, _render_resources| {
                                for (handle, mut renderable) in write_handle_query.iter_mut(world) {
                                    if !renderable.is_visible {
                                        return;
                                    }
                                    if renderable.is_instanced {
                                        panic!("instancing not currently supported");
                                    } else {
                                        if let Some(uniforms) = assets.get(&handle) {
                                            // TODO: only setup buffer if we haven't seen this handle before
                                            uniform_buffer_arrays.setup_uniform_buffer_resources(
                                                &uniforms,
                                                dynamic_uniforms,
                                                render_resource_context,
                                                &mut renderable.render_resource_assignments,
                                                &mut staging_buffer,
                                            );
                                        }
                                    }
                                }
                            },
                        );

                        uniform_buffer_arrays.copy_staging_buffer_to_final_buffers(
                            &mut command_queue,
                            staging_buffer,
                        );
                        command_queue.free_buffer(staging_buffer);
                    }
                },
            )
    }
}

#[allow(dead_code)]
fn initialize_vertex_buffer_descriptor<T>(vertex_buffer_descriptors: &mut VertexBufferDescriptors)
where
    T: AsUniforms,
{
    let vertex_buffer_descriptor = T::get_vertex_buffer_descriptor();
    if let Some(vertex_buffer_descriptor) = vertex_buffer_descriptor {
        if let None = vertex_buffer_descriptors.get(&vertex_buffer_descriptor.name) {
            vertex_buffer_descriptors.set(vertex_buffer_descriptor.clone());
        }
    }
}

fn setup_uniform_texture_resources<T>(
    entity: Entity,
    uniforms: &T,
    render_resource_context: &dyn RenderResourceContext,
    entities_waiting_for_assets: &EntitiesWaitingForAssets,
    render_resource_assignments: &mut RenderResourceAssignments,
) where
    T: AsUniforms,
{
    for field_info in T::get_field_infos().iter() {
        let bind_type = uniforms.get_field_bind_type(&field_info.name);
        if let Some(FieldBindType::Texture) = bind_type {
            if let Some(texture_handle) = uniforms.get_uniform_texture(&field_info.texture_name) {
                if let Some(texture_resource) = render_resource_context
                    .get_asset_resource(texture_handle, texture::TEXTURE_ASSET_INDEX)
                {
                    let sampler_resource = render_resource_context
                        .get_asset_resource(texture_handle, texture::SAMPLER_ASSET_INDEX)
                        .unwrap();
                    render_resource_assignments.set(
                        field_info.texture_name,
                        RenderResourceAssignment::Texture(texture_resource),
                    );
                    render_resource_assignments.set(
                        field_info.sampler_name,
                        RenderResourceAssignment::Sampler(sampler_resource),
                    );
                    continue;
                } else {
                    entities_waiting_for_assets.add(entity);
                }
            } else {
                entities_waiting_for_assets.add(entity);
            }
        }
    }
}
