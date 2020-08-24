use crate::{
    draw::Draw,
    pipeline::RenderPipelines,
    render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
    renderer::{
        self, BufferInfo, BufferUsage, RenderContext, RenderResourceBinding,
        RenderResourceBindings, RenderResourceBindingsId, RenderResourceContext,
        RenderResourceHints,
    },
    texture,
};

use bevy_asset::{Assets, Handle};
use bevy_ecs::{Commands, IntoQuerySystem, Local, Query, Res, ResMut, Resources, System, World};
use renderer::{AssetRenderResourceBindings, BufferId, RenderResourceType, RenderResources};
use std::{collections::HashMap, marker::PhantomData, ops::DerefMut};

pub const BIND_BUFFER_ALIGNMENT: usize = 256;
#[derive(Debug)]
struct QueuedBufferWrite {
    buffer: BufferId,
    target_offset: usize,
    source_offset: usize,
    size: usize,
}

#[derive(Debug)]
struct BufferArrayStatus {
    changed_item_count: usize,
    item_size: usize,
    aligned_size: usize,
    staging_buffer_offset: usize,
    buffer: Option<BufferId>,
    queued_buffer_writes: Vec<QueuedBufferWrite>,
    current_item_count: usize,
    current_item_capacity: usize,
    indices: HashMap<RenderResourceBindingsId, usize>,
    current_index: usize,
    // TODO: this is a hack to workaround RenderResources without a fixed length
    changed_size: usize,
    current_offset: usize,
}

impl BufferArrayStatus {
    pub fn get_or_assign_index(&mut self, id: RenderResourceBindingsId) -> usize {
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
    T: renderer::RenderResources,
{
    uniform_arrays: Vec<Option<(String, BufferArrayStatus)>>,
    staging_buffer: Option<BufferId>,
    staging_buffer_size: usize,
    _marker: PhantomData<T>,
}

impl<T> Default for UniformBufferArrays<T>
where
    T: renderer::RenderResources,
{
    fn default() -> Self {
        Self {
            uniform_arrays: Default::default(),
            staging_buffer: Default::default(),
            staging_buffer_size: 0,
            _marker: Default::default(),
        }
    }
}

impl<T> UniformBufferArrays<T>
where
    T: renderer::RenderResources,
{
    fn reset_changed_item_counts(&mut self) {
        for buffer_status in self.uniform_arrays.iter_mut() {
            if let Some((_name, buffer_status)) = buffer_status {
                buffer_status.changed_item_count = 0;
                buffer_status.current_index = 0;
                buffer_status.indices.clear();
                buffer_status.current_offset = 0;
                buffer_status.changed_size = 0;
            }
        }
    }

    fn increment_changed_item_counts(&mut self, uniforms: &T) {
        if self.uniform_arrays.len() != uniforms.render_resources_len() {
            self.uniform_arrays
                .resize_with(uniforms.render_resources_len(), || None);
        }
        for (i, render_resource) in uniforms.iter_render_resources().enumerate() {
            if let Some(RenderResourceType::Buffer) = render_resource.resource_type() {
                let render_resource_name = uniforms.get_render_resource_name(i).unwrap();
                let size = render_resource.buffer_byte_len().unwrap();
                if let Some((ref _name, ref mut buffer_array_status)) = self.uniform_arrays[i] {
                    buffer_array_status.changed_item_count += 1;
                    buffer_array_status.changed_size += size;
                } else {
                    self.uniform_arrays[i] = Some((
                        render_resource_name.to_string(),
                        BufferArrayStatus {
                            changed_item_count: 1,
                            queued_buffer_writes: Vec::new(),
                            aligned_size: Self::get_aligned_dynamic_uniform_size(size),
                            item_size: size,
                            staging_buffer_offset: 0,
                            buffer: None,
                            current_index: 0,
                            current_item_count: 0,
                            current_item_capacity: 0,
                            indices: HashMap::new(),
                            changed_size: size,
                            current_offset: 0,
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
                    Vec::with_capacity(buffer_array_status.changed_item_count);
            }
        }
    }

    fn setup_buffer_array(
        buffer_array_status: &mut BufferArrayStatus,
        render_resource_context: &dyn RenderResourceContext,
        align: bool,
    ) {
        if buffer_array_status.current_item_capacity < buffer_array_status.changed_item_count {
            let new_capacity =
                buffer_array_status.changed_item_count + buffer_array_status.changed_item_count / 2;
            let mut item_size = buffer_array_status.item_size;
            if align {
                item_size = Self::get_aligned_dynamic_uniform_size(item_size);
            }

            let total_size = item_size * new_capacity;

            let buffer = render_resource_context.create_buffer(BufferInfo {
                size: total_size,
                buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                ..Default::default()
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

    fn update_staging_buffer(&mut self, render_resource_context: &dyn RenderResourceContext) {
        let mut size = 0;
        for dynamic_buffer_array_status in self.uniform_arrays.iter_mut() {
            if let Some((_name, ref mut buffer_array_status)) = dynamic_buffer_array_status {
                buffer_array_status.staging_buffer_offset = size;
                size += buffer_array_status.changed_size;
            }
        }

        if self.staging_buffer_size != size {
            if let Some(staging_buffer) = self.staging_buffer {
                render_resource_context.remove_buffer(staging_buffer);
            }

            if size > 0 {
                let staging_buffer = render_resource_context.create_buffer(BufferInfo {
                    buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                    size,
                    ..Default::default()
                });
                self.staging_buffer = Some(staging_buffer);
            } else {
                self.staging_buffer = None;
            }

            self.staging_buffer_size = size;
        }
    }

    fn setup_uniform_buffer_resources(
        &mut self,
        uniforms: &T,
        dynamic_uniforms: bool,
        render_resource_context: &dyn RenderResourceContext,
        render_resource_bindings: &mut RenderResourceBindings,
        staging_buffer: &mut [u8],
    ) {
        for (i, render_resource) in uniforms.iter_render_resources().enumerate() {
            match render_resource.resource_type() {
                Some(RenderResourceType::Buffer) => {
                    let size = render_resource.buffer_byte_len().unwrap();
                    let render_resource_name = uniforms.get_render_resource_name(i).unwrap();
                    let (_name, uniform_buffer_status) = self.uniform_arrays[i].as_mut().unwrap();
                    let range = 0..size as u64;
                    let (target_buffer, target_offset) = if dynamic_uniforms {
                        let buffer = uniform_buffer_status.buffer.unwrap();
                        let index =
                            uniform_buffer_status.get_or_assign_index(render_resource_bindings.id);
                        render_resource_bindings.set(
                            render_resource_name,
                            RenderResourceBinding::Buffer {
                                buffer,
                                dynamic_index: Some(
                                    (index * uniform_buffer_status.aligned_size) as u32,
                                ),
                                range,
                            },
                        );
                        (buffer, index * uniform_buffer_status.aligned_size)
                    } else {
                        let mut matching_buffer = None;
                        if let Some(binding) = render_resource_bindings.get(render_resource_name) {
                            let buffer_id = binding.get_buffer().unwrap();
                            if let Some(BufferInfo {
                                size: current_size, ..
                            }) = render_resource_context.get_buffer_info(buffer_id)
                            {
                                if size == current_size {
                                    matching_buffer = Some(buffer_id);
                                } else {
                                    render_resource_context.remove_buffer(buffer_id);
                                }
                            }
                        }

                        let resource = if let Some(matching_buffer) = matching_buffer {
                            matching_buffer
                        } else {
                            let mut usage = BufferUsage::UNIFORM;
                            if let Some(render_resource_hints) =
                                uniforms.get_render_resource_hints(i)
                            {
                                if render_resource_hints.contains(RenderResourceHints::BUFFER) {
                                    usage = BufferUsage::STORAGE
                                }
                            }

                            let buffer = render_resource_context.create_buffer(BufferInfo {
                                size,
                                buffer_usage: BufferUsage::COPY_DST | usage,
                                ..Default::default()
                            });

                            render_resource_bindings.set(
                                render_resource_name,
                                RenderResourceBinding::Buffer {
                                    buffer,
                                    range,
                                    dynamic_index: None,
                                },
                            );
                            buffer
                        };

                        (resource, 0)
                    };

                    let staging_buffer_start = uniform_buffer_status.staging_buffer_offset
                        + uniform_buffer_status.current_offset;

                    render_resource.write_buffer_bytes(
                        &mut staging_buffer[staging_buffer_start..(staging_buffer_start + size)],
                    );

                    uniform_buffer_status
                        .queued_buffer_writes
                        .push(QueuedBufferWrite {
                            buffer: target_buffer,
                            target_offset,
                            source_offset: uniform_buffer_status.current_offset,
                            size,
                        });
                    uniform_buffer_status.current_offset += size;
                }
                Some(RenderResourceType::Texture) => { /* ignore textures */ }
                Some(RenderResourceType::Sampler) => { /* ignore samplers */ }
                None => { /* ignore None */ }
            }
        }
    }

    fn copy_staging_buffer_to_final_buffers(
        &mut self,
        command_queue: &mut CommandQueue,
        staging_buffer: BufferId,
    ) {
        for uniform_buffer_status in self.uniform_arrays.iter_mut() {
            if let Some((_name, buffer_array_status)) = uniform_buffer_status {
                let start = buffer_array_status.staging_buffer_offset;
                for queued_buffer_write in buffer_array_status.queued_buffer_writes.drain(..) {
                    command_queue.copy_buffer_to_buffer(
                        staging_buffer,
                        (start + queued_buffer_write.source_offset) as u64,
                        queued_buffer_write.buffer,
                        queued_buffer_write.target_offset as u64,
                        queued_buffer_write.size as u64,
                    )
                }
            }
        }
    }
}

#[derive(Default)]
pub struct RenderResourcesNode<T>
where
    T: renderer::RenderResources,
{
    command_queue: CommandQueue,
    dynamic_uniforms: bool,
    _marker: PhantomData<T>,
}

impl<T> RenderResourcesNode<T>
where
    T: renderer::RenderResources,
{
    pub fn new(dynamic_uniforms: bool) -> Self {
        RenderResourcesNode {
            command_queue: CommandQueue::default(),
            dynamic_uniforms,
            _marker: PhantomData::default(),
        }
    }
}

impl<T> Node for RenderResourcesNode<T>
where
    T: renderer::RenderResources,
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

impl<T> SystemNode for RenderResourcesNode<T>
where
    T: renderer::RenderResources,
{
    fn get_system(&self, commands: &mut Commands) -> Box<dyn System> {
        let system = render_resources_node_system::<T>.system();
        commands.insert_local_resource(
            system.id(),
            RenderResourcesNodeState {
                command_queue: self.command_queue.clone(),
                uniform_buffer_arrays: UniformBufferArrays::<T>::default(),
                dynamic_uniforms: self.dynamic_uniforms,
            },
        );

        system
    }
}

struct RenderResourcesNodeState<T: RenderResources> {
    command_queue: CommandQueue,
    uniform_buffer_arrays: UniformBufferArrays<T>,
    dynamic_uniforms: bool,
}

impl<T: RenderResources> Default for RenderResourcesNodeState<T> {
    fn default() -> Self {
        Self {
            command_queue: Default::default(),
            uniform_buffer_arrays: Default::default(),
            dynamic_uniforms: Default::default(),
        }
    }
}

fn render_resources_node_system<T: RenderResources>(
    mut state: Local<RenderResourcesNodeState<T>>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut query: Query<(&T, &Draw, &mut RenderPipelines)>,
) {
    let state = state.deref_mut();
    let render_resource_context = &**render_resource_context;
    state.uniform_buffer_arrays.reset_changed_item_counts();
    // update uniforms info
    for (uniforms, draw, _render_pipelines) in &mut query.iter() {
        if !draw.is_visible {
            continue;
        }

        state
            .uniform_buffer_arrays
            .increment_changed_item_counts(&uniforms);
    }
    state
        .uniform_buffer_arrays
        .setup_buffer_arrays(render_resource_context, state.dynamic_uniforms);
    state
        .uniform_buffer_arrays
        .update_staging_buffer(render_resource_context);

    for (uniforms, draw, mut render_pipelines) in &mut query.iter() {
        if !draw.is_visible {
            continue;
        }

        setup_uniform_texture_resources::<T>(
            &uniforms,
            render_resource_context,
            &mut render_pipelines.bindings,
        )
    }

    if let Some(staging_buffer) = state.uniform_buffer_arrays.staging_buffer {
        render_resource_context.map_buffer(staging_buffer);
        render_resource_context.write_mapped_buffer(
            staging_buffer,
            0..state.uniform_buffer_arrays.staging_buffer_size as u64,
            &mut |mut staging_buffer, _render_resource_context| {
                for (uniforms, draw, mut render_pipelines) in &mut query.iter() {
                    if !draw.is_visible {
                        continue;
                    }

                    state.uniform_buffer_arrays.setup_uniform_buffer_resources(
                        &uniforms,
                        state.dynamic_uniforms,
                        render_resource_context,
                        &mut render_pipelines.bindings,
                        &mut staging_buffer,
                    );
                }
            },
        );
        render_resource_context.unmap_buffer(staging_buffer);

        state
            .uniform_buffer_arrays
            .copy_staging_buffer_to_final_buffers(&mut state.command_queue, staging_buffer);
    } else {
        // TODO: can we just remove this?
        let mut staging_buffer: [u8; 0] = [];
        for (uniforms, draw, mut render_pipelines) in &mut query.iter() {
            if !draw.is_visible {
                continue;
            }

            state.uniform_buffer_arrays.setup_uniform_buffer_resources(
                &uniforms,
                state.dynamic_uniforms,
                render_resource_context,
                &mut render_pipelines.bindings,
                &mut staging_buffer,
            );
        }
    }
}

#[derive(Default)]
pub struct AssetRenderResourcesNode<T>
where
    T: renderer::RenderResources,
{
    command_queue: CommandQueue,
    dynamic_uniforms: bool,
    _marker: PhantomData<T>,
}

impl<T> AssetRenderResourcesNode<T>
where
    T: renderer::RenderResources,
{
    pub fn new(dynamic_uniforms: bool) -> Self {
        AssetRenderResourcesNode {
            dynamic_uniforms,
            command_queue: Default::default(),
            _marker: Default::default(),
        }
    }
}

impl<T> Node for AssetRenderResourcesNode<T>
where
    T: renderer::RenderResources,
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

const EXPECT_ASSET_MESSAGE: &str = "Only assets that exist should be in the modified assets list";

impl<T> SystemNode for AssetRenderResourcesNode<T>
where
    T: renderer::RenderResources,
{
    fn get_system(&self, commands: &mut Commands) -> Box<dyn System> {
        let system = asset_render_resources_node_system::<T>.system();
        commands.insert_local_resource(
            system.id(),
            RenderResourcesNodeState {
                command_queue: self.command_queue.clone(),
                uniform_buffer_arrays: UniformBufferArrays::<T>::default(),
                dynamic_uniforms: self.dynamic_uniforms,
            },
        );

        system
    }
}

fn asset_render_resources_node_system<T: RenderResources>(
    mut state: Local<RenderResourcesNodeState<T>>,
    assets: Res<Assets<T>>,
    //    asset_events: Res<Events<AssetEvent<T>>>,
    mut asset_render_resource_bindings: ResMut<AssetRenderResourceBindings>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut query: Query<(&Handle<T>, &Draw, &mut RenderPipelines)>,
) {
    let state = state.deref_mut();
    let render_resource_context = &**render_resource_context;
    state.uniform_buffer_arrays.reset_changed_item_counts();

    let modified_assets = assets
        .iter()
        .map(|(handle, _)| handle)
        .collect::<Vec<Handle<T>>>();
    // TODO: uncomment this when asset dependency events are added https://github.com/bevyengine/bevy/issues/26
    // let mut modified_assets = HashSet::new();
    // for event in asset_event_reader.iter(&asset_events) {
    //     match event {
    //         AssetEvent::Created { handle } => {
    //             modified_assets.insert(*handle);
    //         }
    //         AssetEvent::Modified { handle } => {
    //             modified_assets.insert(*handle);
    //         }
    //         AssetEvent::Removed { handle } => {
    //             // TODO: handle removals
    //             modified_assets.remove(handle);
    //         }
    //     }
    // }

    // update uniform handles info
    for asset_handle in modified_assets.iter() {
        let asset = assets.get(&asset_handle).expect(EXPECT_ASSET_MESSAGE);
        state
            .uniform_buffer_arrays
            .increment_changed_item_counts(&asset);
    }

    state
        .uniform_buffer_arrays
        .setup_buffer_arrays(render_resource_context, state.dynamic_uniforms);
    state
        .uniform_buffer_arrays
        .update_staging_buffer(render_resource_context);

    for asset_handle in modified_assets.iter() {
        let asset = assets.get(&asset_handle).expect(EXPECT_ASSET_MESSAGE);
        let mut render_resource_bindings =
            asset_render_resource_bindings.get_or_insert_mut(*asset_handle);
        setup_uniform_texture_resources::<T>(
            &asset,
            render_resource_context,
            &mut render_resource_bindings,
        );
    }

    if let Some(staging_buffer) = state.uniform_buffer_arrays.staging_buffer {
        render_resource_context.map_buffer(staging_buffer);
        render_resource_context.write_mapped_buffer(
            staging_buffer,
            0..state.uniform_buffer_arrays.staging_buffer_size as u64,
            &mut |mut staging_buffer, _render_resource_context| {
                for asset_handle in modified_assets.iter() {
                    let asset = assets.get(&asset_handle).expect(EXPECT_ASSET_MESSAGE);
                    let mut render_resource_bindings =
                        asset_render_resource_bindings.get_or_insert_mut(*asset_handle);
                    // TODO: only setup buffer if we haven't seen this handle before
                    state.uniform_buffer_arrays.setup_uniform_buffer_resources(
                        &asset,
                        state.dynamic_uniforms,
                        render_resource_context,
                        &mut render_resource_bindings,
                        &mut staging_buffer,
                    );
                }
            },
        );
        render_resource_context.unmap_buffer(staging_buffer);

        state
            .uniform_buffer_arrays
            .copy_staging_buffer_to_final_buffers(&mut state.command_queue, staging_buffer);
    } else {
        let mut staging_buffer: [u8; 0] = [];
        for asset_handle in modified_assets.iter() {
            let asset = assets.get(&asset_handle).expect(EXPECT_ASSET_MESSAGE);
            let mut render_resource_bindings =
                asset_render_resource_bindings.get_or_insert_mut(*asset_handle);
            // TODO: only setup buffer if we haven't seen this handle before
            state.uniform_buffer_arrays.setup_uniform_buffer_resources(
                &asset,
                state.dynamic_uniforms,
                render_resource_context,
                &mut render_resource_bindings,
                &mut staging_buffer,
            );
        }
    }

    for (asset_handle, draw, mut render_pipelines) in &mut query.iter() {
        if !draw.is_visible {
            continue;
        }
        if let Some(asset_bindings) = asset_render_resource_bindings.get(*asset_handle) {
            render_pipelines.bindings.extend(asset_bindings);
        }
    }
}

fn setup_uniform_texture_resources<T>(
    uniforms: &T,
    render_resource_context: &dyn RenderResourceContext,
    render_resource_bindings: &mut RenderResourceBindings,
) where
    T: renderer::RenderResources,
{
    for (i, render_resource) in uniforms.iter_render_resources().enumerate() {
        if let Some(RenderResourceType::Texture) = render_resource.resource_type() {
            let render_resource_name = uniforms.get_render_resource_name(i).unwrap();
            let sampler_name = format!("{}_sampler", render_resource_name);
            if let Some(texture_handle) = render_resource.texture() {
                if let Some(texture_resource) = render_resource_context
                    .get_asset_resource(texture_handle, texture::TEXTURE_ASSET_INDEX)
                {
                    let sampler_resource = render_resource_context
                        .get_asset_resource(texture_handle, texture::SAMPLER_ASSET_INDEX)
                        .unwrap();

                    render_resource_bindings.set(
                        render_resource_name,
                        RenderResourceBinding::Texture(texture_resource.get_texture().unwrap()),
                    );
                    render_resource_bindings.set(
                        &sampler_name,
                        RenderResourceBinding::Sampler(sampler_resource.get_sampler().unwrap()),
                    );
                    continue;
                }
            }
        }
    }
}
