use crate::{
    pipeline::VertexBufferDescriptors,
    render_resource::{
        AssetBatchers, BufferArrayInfo, BufferInfo, BufferUsage, RenderResource,
        RenderResourceAssignments, RenderResourceAssignmentsId, ResourceInfo, ResourceProvider,
    },
    renderer_2::{RenderContext, RenderResourceContext},
    shader::{AsUniforms, FieldBindType},
    texture::{self, SamplerDescriptor, Texture, TextureDescriptor},
    Renderable,
};
use bevy_asset::{AssetStorage, Handle};
use legion::{filter::*, prelude::*};
use std::{collections::HashMap, marker::PhantomData};
pub const BIND_BUFFER_ALIGNMENT: usize = 256;

#[derive(Debug)]
struct BufferArrayStatus {
    new_item_count: usize,
    item_size: usize,
    aligned_size: usize,
    staging_buffer_offset: usize,
    queued_buffer_writes: Vec<QueuedBufferWrite>,
    buffer: Option<RenderResource>,

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

#[derive(Debug)]
struct QueuedBufferWrite {
    buffer: RenderResource,
    offset: usize,
}

// TODO: make these queries only update changed T components
type UniformQueryRead<T> = Query<
    (Read<T>, Read<Renderable>),
    EntityFilterTuple<
        And<(ComponentFilter<T>, ComponentFilter<Renderable>)>,
        And<(Passthrough, Passthrough)>,
        And<(Passthrough, Passthrough)>,
    >,
>;

type UniformQuery<T> = Query<
    (Read<T>, Write<Renderable>),
    EntityFilterTuple<
        And<(ComponentFilter<T>, ComponentFilter<Renderable>)>,
        And<(Passthrough, Passthrough)>,
        And<(Passthrough, Passthrough)>,
    >,
>;

type UniformHandleQueryRead<T> = Query<
    (Read<Handle<T>>, Read<Renderable>),
    EntityFilterTuple<
        And<(ComponentFilter<Handle<T>>, ComponentFilter<Renderable>)>,
        And<(Passthrough, Passthrough)>,
        And<(Passthrough, Passthrough)>,
    >,
>;

type UniformHandleQuery<T> = Query<
    (Read<Handle<T>>, Write<Renderable>),
    EntityFilterTuple<
        And<(ComponentFilter<Handle<T>>, ComponentFilter<Renderable>)>,
        And<(Passthrough, Passthrough)>,
        And<(Passthrough, Passthrough)>,
    >,
>;

// TODO: rename to RenderResourceProvider
pub struct UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync + 'static,
{
    _marker: PhantomData<T>,
    use_dynamic_uniforms: bool,
    is_instanceable: bool,
    // PERF: somehow remove this HashSet
    uniform_buffer_status: Vec<Option<(String, BufferArrayStatus)>>,
    instance_buffer_status: Option<BufferArrayStatus>,
    query: Option<UniformQueryRead<T>>,
    query_finish: Option<UniformQuery<T>>,
    handle_query: Option<UniformHandleQueryRead<T>>,
    handle_query_finish: Option<UniformHandleQuery<T>>,
}

impl<T> UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync + 'static,
{
    pub fn new(use_dynamic_uniforms: bool) -> Self {
        let mut dynamic_uniform_buffer_status = Vec::new();
        let field_infos = T::get_field_infos();
        dynamic_uniform_buffer_status.resize_with(field_infos.len(), || None);
        let is_instanceable = field_infos.iter().find(|f| f.is_instanceable).is_some();
        UniformResourceProvider {
            uniform_buffer_status: dynamic_uniform_buffer_status,
            use_dynamic_uniforms,
            instance_buffer_status: None,
            is_instanceable,
            query: Some(<(Read<T>, Read<Renderable>)>::query()),
            query_finish: Some(<(Read<T>, Write<Renderable>)>::query()),
            handle_query: Some(<(Read<Handle<T>>, Read<Renderable>)>::query()),
            handle_query_finish: Some(<(Read<Handle<T>>, Write<Renderable>)>::query()),
            _marker: PhantomData,
        }
    }

    fn reset_buffer_array_status_counts(&mut self) {
        for buffer_status in self.uniform_buffer_status.iter_mut() {
            if let Some((_name, buffer_status)) = buffer_status {
                buffer_status.new_item_count = 0;
            }
        }

        if let Some(ref mut buffer_status) = self.instance_buffer_status {
            buffer_status.new_item_count = 0;
        }
    }

    fn update_uniforms_info(&mut self, world: &World) {
        let query = self.query.take().unwrap();
        for (uniforms, renderable) in query.iter(world) {
            if !renderable.is_visible {
                return;
            }

            if renderable.is_instanced {
                if self.is_instanceable {
                    self.increment_instance_count(|| Self::get_instance_size(&uniforms));
                } else {
                    panic!(
                        "Cannot instance uniforms of type {}",
                        std::any::type_name::<T>()
                    );
                }
            } else {
                self.increment_uniform_counts(&uniforms);
            }
        }

        self.query = Some(query);
    }

    fn update_uniform_handles_info(&mut self, world: &World, resources: &Resources) {
        let handle_query = self.handle_query.take().unwrap();
        let assets = resources.get::<AssetStorage<T>>();
        if let Some(assets) = assets {
            for (handle, renderable) in handle_query.iter(world) {
                if !renderable.is_visible {
                    return;
                }

                if renderable.is_instanced {
                    if self.is_instanceable {
                        self.increment_instance_count(|| {
                            let uniforms = assets.get(&handle).unwrap();
                            Self::get_instance_size(uniforms)
                        });
                    } else {
                        panic!(
                            "Cannot instance uniforms of type Handle<{}>",
                            std::any::type_name::<T>()
                        );
                    }
                } else {
                    let uniforms = assets
                        .get(&handle)
                        .expect("Handle points to a non-existent resource");
                    // TODO: only increment count if we haven't seen this uniform handle before
                    self.increment_uniform_counts(&uniforms);
                }
            }
        }

        self.handle_query = Some(handle_query);
    }

    fn increment_instance_count(&mut self, f: impl Fn() -> usize) {
        if let Some(ref mut buffer_array_status) = self.instance_buffer_status {
            buffer_array_status.new_item_count += 1;
        } else {
            self.instance_buffer_status = Some(BufferArrayStatus {
                new_item_count: 1,
                queued_buffer_writes: Vec::new(),
                aligned_size: 0,
                item_size: f(),
                staging_buffer_offset: 0,
                buffer: None,
                current_index: 0,
                current_item_capacity: 0,
                current_item_count: 0,
                indices: HashMap::new(),
            })
        }
    }

    fn get_instance_size(uniforms: &T) -> usize {
        let mut instance_buffer_size = 0;
        for field_info in T::get_field_infos().iter() {
            if field_info.is_instanceable {
                if let Some(FieldBindType::Uniform { size }) =
                    uniforms.get_field_bind_type(field_info.name)
                {
                    instance_buffer_size += size;
                }
            }
        }

        instance_buffer_size
    }

    fn increment_uniform_counts(&mut self, uniforms: &T) {
        for (i, field_info) in T::get_field_infos().iter().enumerate() {
            if let Some(FieldBindType::Uniform { size }) =
                uniforms.get_field_bind_type(&field_info.name)
            {
                if let Some((ref _name, ref mut buffer_array_status)) =
                    self.uniform_buffer_status[i]
                {
                    buffer_array_status.new_item_count += 1;
                } else {
                    self.uniform_buffer_status[i] = Some((
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

    fn setup_uniform_buffer_resources(
        &mut self,
        uniforms: &T,
        render_resources: &mut dyn RenderResourceContext,
        render_resource_assignments: &mut RenderResourceAssignments,
        staging_buffer: &mut [u8],
    ) {
        for (i, field_info) in T::get_field_infos().iter().enumerate() {
            let bind_type = uniforms.get_field_bind_type(&field_info.name);
            match bind_type {
                Some(FieldBindType::Uniform { size }) => {
                    let (_name, uniform_buffer_status) =
                        self.uniform_buffer_status[i].as_mut().unwrap();
                    let (target_buffer, target_offset) = if self.use_dynamic_uniforms {
                        let buffer = uniform_buffer_status.buffer.unwrap();
                        let mut offset = 0;
                        render_resources.get_resource_info(buffer, &mut |resource_info| {
                            if let Some(ResourceInfo::Buffer(BufferInfo {
                                array_info: Some(ref array_info),
                                is_dynamic: true,
                                ..
                            })) = resource_info
                            {
                                let index = uniform_buffer_status
                                    .get_or_assign_index(render_resource_assignments.id);
                                render_resource_assignments.set_indexed(
                                    &field_info.uniform_name,
                                    buffer,
                                    (index * array_info.item_size) as u32,
                                );
                                offset = index * uniform_buffer_status.aligned_size;
                            } else {
                                panic!("Expected a dynamic uniform buffer");
                            }
                        });
                        (buffer, offset)
                    } else {
                        let resource = match render_resource_assignments
                            .get(field_info.uniform_name)
                        {
                            Some(render_resource) => render_resource,
                            None => {
                                let resource = render_resources.create_buffer(BufferInfo {
                                    size,
                                    buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                                    ..Default::default()
                                });
                                render_resource_assignments.set(&field_info.uniform_name, resource);
                                resource
                            }
                        };

                        (resource, 0)
                    };

                    let staging_buffer_start = uniform_buffer_status.staging_buffer_offset
                        + (uniform_buffer_status.queued_buffer_writes.len()
                            * uniform_buffer_status.item_size);
                    if let Some(uniform_bytes) =
                        uniforms.get_uniform_bytes_ref(&field_info.uniform_name)
                    {
                        if size != uniform_bytes.len() {
                            panic!("The number of bytes produced for {} do not match the expected count. Actual: {}. Expected: {}.", field_info.uniform_name, uniform_bytes.len(), size);
                        }

                        staging_buffer
                            [staging_buffer_start..(staging_buffer_start + uniform_bytes.len())]
                            .copy_from_slice(uniform_bytes);
                    } else if let Some(uniform_bytes) =
                        uniforms.get_uniform_bytes(field_info.uniform_name)
                    {
                        if size != uniform_bytes.len() {
                            panic!("The number of bytes produced for {} do not match the expected count. Actual: {}. Expected: {}.", field_info.uniform_name, uniform_bytes.len(), size);
                        }

                        staging_buffer
                            [staging_buffer_start..(staging_buffer_start + uniform_bytes.len())]
                            .copy_from_slice(&uniform_bytes);
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
                _ => {}
            }
        }
    }

    fn setup_uniform_texture_resources(
        &mut self,
        uniforms: &T,
        render_context: &mut dyn RenderContext,
        resources: &Resources,
        render_resource_assignments: &mut RenderResourceAssignments,
    ) {
        for field_info in T::get_field_infos().iter() {
            let bind_type = uniforms.get_field_bind_type(&field_info.name);
            match bind_type {
                Some(FieldBindType::Texture) => {
                    let texture_handle = uniforms
                        .get_uniform_texture(&field_info.texture_name)
                        .unwrap();
                    let (texture_resource, sampler_resource) = match render_context
                        .resources()
                        .get_asset_resource(texture_handle, texture::TEXTURE_ASSET_INDEX)
                    {
                        Some(texture_resource) => (
                            texture_resource,
                            render_context
                                .resources()
                                .get_asset_resource(texture_handle, texture::SAMPLER_ASSET_INDEX)
                                .unwrap(),
                        ),
                        None => {
                            let storage = resources.get::<AssetStorage<Texture>>().unwrap();
                            let texture = storage.get(&texture_handle).unwrap();

                            let texture_descriptor: TextureDescriptor = texture.into();
                            let texture_resource = render_context
                                .create_texture_with_data(&texture_descriptor, &texture.data);

                            let render_resources = render_context.resources_mut();
                            let sampler_descriptor: SamplerDescriptor = texture.into();
                            let sampler_resource =
                                render_resources.create_sampler(&sampler_descriptor);

                            render_resources.set_asset_resource(
                                texture_handle,
                                texture_resource,
                                0,
                            );
                            render_resources.set_asset_resource(
                                texture_handle,
                                sampler_resource,
                                1,
                            );
                            (texture_resource, sampler_resource)
                        }
                    };

                    render_resource_assignments.set(field_info.texture_name, texture_resource);
                    render_resource_assignments.set(field_info.sampler_name, sampler_resource);
                }
                _ => {}
            }
        }
    }

    // TODO: the current WgpuRenderContext mapped-memory interface forced these to be separate, but thats inefficient / redundant
    // try to merge setup_uniforms_buffer_resources and setup_uniforms_texture_resources if possible
    fn setup_uniforms_buffer_resources(
        &mut self,
        world: &mut World,
        render_resources: &mut dyn RenderResourceContext,
        staging_buffer: &mut [u8],
    ) {
        let query_finish = self.query_finish.take().unwrap();
        for (uniforms, mut renderable) in query_finish.iter_mut(world) {
            if !renderable.is_visible {
                return;
            }

            if renderable.is_instanced {
                panic!(
                    "Cannot instance uniforms of type {0}. Only Handle<{0}> can be instanced.",
                    std::any::type_name::<T>()
                );
            } else {
                self.setup_uniform_buffer_resources(
                    &uniforms,
                    render_resources,
                    &mut renderable.render_resource_assignments,
                    staging_buffer,
                );
            }
        }

        self.query_finish = Some(query_finish);
    }

    fn setup_uniforms_texture_resources(
        &mut self,
        world: &mut World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
    ) {
        let query_finish = self.query_finish.take().unwrap();
        for (uniforms, mut renderable) in query_finish.iter_mut(world) {
            if !renderable.is_visible {
                return;
            }

            if renderable.is_instanced {
                panic!(
                    "Cannot instance uniforms of type {0}. Only Handle<{0}> can be instanced.",
                    std::any::type_name::<T>()
                );
            } else {
                self.setup_uniform_texture_resources(
                    &uniforms,
                    render_context,
                    resources,
                    &mut renderable.render_resource_assignments,
                )
            }
        }

        self.query_finish = Some(query_finish);
    }

    fn setup_handles_buffer_resources(
        &mut self,
        world: &mut World,
        resources: &Resources,
        render_resources: &mut dyn RenderResourceContext,
        staging_buffer: &mut [u8],
    ) {
        let assets = resources.get::<AssetStorage<T>>();
        if let Some(assets) = assets {
            let handle_query_finish = self.handle_query_finish.take().unwrap();
            for (handle, mut renderable) in handle_query_finish.iter_mut(world) {
                if !renderable.is_visible || renderable.is_instanced {
                    return;
                }

                let uniforms = assets
                    .get(&handle)
                    .expect("Handle points to a non-existent resource");
                // TODO: only setup buffer if we haven't seen this handle before
                self.setup_uniform_buffer_resources(
                    &uniforms,
                    render_resources,
                    &mut renderable.render_resource_assignments,
                    staging_buffer,
                );
            }

            self.handle_query_finish = Some(handle_query_finish);
        }
    }

    fn setup_handles_texture_resources(
        &mut self,
        world: &mut World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
    ) {
        let assets = resources.get::<AssetStorage<T>>();
        if let Some(assets) = assets {
            let handle_query_finish = self.handle_query_finish.take().unwrap();
            for (handle, mut renderable) in handle_query_finish.iter_mut(world) {
                if !renderable.is_visible || renderable.is_instanced {
                    return;
                }

                let uniforms = assets
                    .get(&handle)
                    .expect("Handle points to a non-existent resource");
                self.setup_uniform_texture_resources(
                    &uniforms,
                    render_context,
                    resources,
                    &mut renderable.render_resource_assignments,
                )
            }

            self.handle_query_finish = Some(handle_query_finish);
        }
    }

    #[allow(dead_code)]
    fn setup_batched_resources(
        &mut self,
        _world: &mut World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        staging_buffer: &mut [u8],
    ) {
        // update batch resources. this needs to run in "finish_update" because batches aren't finalized across
        // all members of the batch until "UniformResourceProvider.update" has run for all members of the batch
        if let Some(asset_storage) = resources.get::<AssetStorage<T>>() {
            let mut asset_batchers = resources.get_mut::<AssetBatchers>().unwrap();
            let handle_type = std::any::TypeId::of::<T>();
            for batch in asset_batchers.get_handle_batches_mut::<T>().unwrap() {
                let handle: Handle<T> = batch
                    .handles
                    .iter()
                    .find(|h| h.type_id == handle_type)
                    .map(|h| (*h).into())
                    .unwrap();

                if let Some(uniforms) = asset_storage.get(&handle) {
                    self.setup_uniform_buffer_resources(
                        &uniforms,
                        render_context.resources_mut(),
                        &mut batch.render_resource_assignments,
                        staging_buffer,
                    );
                    self.setup_uniform_texture_resources(
                        &uniforms,
                        render_context,
                        resources,
                        &mut batch.render_resource_assignments,
                    );
                }
            }
        }
    }

    fn setup_buffer_arrays(&mut self, render_context: &mut dyn RenderContext) {
        for buffer_array_status in self.uniform_buffer_status.iter_mut() {
            if let Some((_name, buffer_array_status)) = buffer_array_status {
                if self.use_dynamic_uniforms {
                    Self::setup_buffer_array(buffer_array_status, render_context, true);
                }

                buffer_array_status.queued_buffer_writes =
                    Vec::with_capacity(buffer_array_status.new_item_count);
            }
        }

        if let Some(ref mut buffer_array_status) = self.instance_buffer_status {
            Self::setup_buffer_array(buffer_array_status, render_context, false);
        }
    }

    fn setup_buffer_array(
        buffer_array_status: &mut BufferArrayStatus,
        render_context: &mut dyn RenderContext,
        align: bool,
    ) {
        let new_capacity = if let Some(buffer) = buffer_array_status.buffer {
            let mut new_capacity = None;
            render_context
                .resources()
                .get_resource_info(buffer, &mut |resource_info| {
                    new_capacity = if let Some(ResourceInfo::Buffer(BufferInfo {
                        array_info: Some(array_info),
                        ..
                    })) = resource_info
                    {
                        if array_info.item_capacity < buffer_array_status.new_item_count {
                            // over capacity. lets resize
                            Some(
                                buffer_array_status.new_item_count
                                    + buffer_array_status.new_item_count / 2,
                            )
                        } else {
                            // under capacity. no change needed
                            None
                        }
                    } else {
                        // incorrect resource type. overwrite with new buffer
                        Some(buffer_array_status.new_item_count)
                    };
                });
            new_capacity
        } else {
            // buffer does not exist. create it now.
            Some(buffer_array_status.new_item_count)
        };

        if let Some(new_capacity) = new_capacity {
            let mut item_size = buffer_array_status.item_size;
            if align {
                item_size = Self::get_aligned_dynamic_uniform_size(item_size);
            }

            let total_size = item_size * new_capacity;

            let buffer = render_context.resources_mut().create_buffer(BufferInfo {
                array_info: Some(BufferArrayInfo {
                    item_capacity: new_capacity,
                    item_size,
                    ..Default::default()
                }),
                size: total_size,
                buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                is_dynamic: true,
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

    fn initialize_vertex_buffer_descriptor(
        &self,
        vertex_buffer_descriptors: &mut VertexBufferDescriptors,
    ) {
        let vertex_buffer_descriptor = T::get_vertex_buffer_descriptor();
        if let Some(vertex_buffer_descriptor) = vertex_buffer_descriptor {
            if let None = vertex_buffer_descriptors.get(&vertex_buffer_descriptor.name) {
                vertex_buffer_descriptors.set(vertex_buffer_descriptor.clone());
            }
        }
    }

    fn update_staging_buffer_offsets(&mut self) -> usize {
        let mut size = 0;
        for dynamic_buffer_array_status in self.uniform_buffer_status.iter_mut() {
            if let Some((_name, ref mut buffer_array_status)) = dynamic_buffer_array_status {
                buffer_array_status.staging_buffer_offset = size;
                size += buffer_array_status.item_size * buffer_array_status.new_item_count;
            }
        }

        size
    }

    fn copy_staging_buffer_to_final_buffers(
        &mut self,
        render_context: &mut dyn RenderContext,
        staging_buffer: RenderResource,
    ) {
        for uniform_buffer_status in self.uniform_buffer_status.iter_mut() {
            if let Some((_name, buffer_array_status)) = uniform_buffer_status {
                let start = buffer_array_status.staging_buffer_offset;
                for (i, queued_buffer_write) in buffer_array_status
                    .queued_buffer_writes
                    .drain(..)
                    .enumerate()
                {
                    render_context.copy_buffer_to_buffer(
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

impl<T> ResourceProvider for UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync + 'static,
{
    fn initialize(
        &mut self,
        render_context: &mut dyn RenderContext,
        world: &mut World,
        resources: &Resources,
    ) {
        let mut vertex_buffer_descriptors = resources.get_mut::<VertexBufferDescriptors>().unwrap();
        self.initialize_vertex_buffer_descriptor(&mut vertex_buffer_descriptors);
        self.update(render_context, world, resources);
    }

    fn update(
        &mut self,
        _render_context: &mut dyn RenderContext,
        world: &World,
        resources: &Resources,
    ) {
        self.reset_buffer_array_status_counts();
        self.update_uniforms_info(world);
        self.update_uniform_handles_info(world, resources);
    }

    fn finish_update(
        &mut self,
        render_context: &mut dyn RenderContext,
        world: &mut World,
        resources: &Resources,
    ) {
        // TODO: when setting batch shader_defs, add INSTANCING
        self.setup_buffer_arrays(render_context);

        let staging_buffer_size = self.update_staging_buffer_offsets();
        self.setup_uniforms_texture_resources(world, resources, render_context);
        self.setup_handles_texture_resources(world, resources, render_context);
        // self.setup_batched_texture_resources(world, resources, renderer, staging_buffer);
        if staging_buffer_size == 0 {
            let mut staging_buffer: [u8; 0] = [];
            self.setup_uniforms_buffer_resources(
                world,
                render_context.resources_mut(),
                &mut staging_buffer,
            );
            self.setup_handles_buffer_resources(
                world,
                resources,
                render_context.resources_mut(),
                &mut staging_buffer,
            );
        // self.setup_batched_buffer_resources(world, resources, renderer, &mut staging_buffer);
        } else {
            let staging_buffer = render_context.resources_mut().create_buffer_mapped(
                BufferInfo {
                    buffer_usage: BufferUsage::COPY_SRC,
                    size: staging_buffer_size,
                    ..Default::default()
                },
                &mut |staging_buffer, render_resources| {
                    self.setup_uniforms_buffer_resources(world, render_resources, staging_buffer);
                    self.setup_handles_buffer_resources(
                        world,
                        resources,
                        render_resources,
                        staging_buffer,
                    );
                    // self.setup_batched_buffer_resources(world, resources, renderer, staging_buffer);
                },
            );

            self.copy_staging_buffer_to_final_buffers(render_context, staging_buffer);
            render_context.resources_mut().remove_buffer(staging_buffer);
        }
    }
}
