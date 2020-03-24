use crate::{
    asset::{AssetStorage, Handle},
    render::{
        render_graph::RenderGraph,
        render_resource::{
            AssetBatchers, BufferArrayInfo, BufferDynamicUniformInfo, BufferInfo, BufferUsage,
            RenderResource, RenderResourceAssignments, RenderResourceAssignmentsProvider,
            ResourceInfo, ResourceProvider,
        },
        renderer::Renderer,
        shader::{AsUniforms, FieldBindType},
        texture::{SamplerDescriptor, Texture, TextureDescriptor},
        Renderable,
    },
};
use legion::{filter::*, prelude::*};
use std::marker::PhantomData;
pub const BIND_BUFFER_ALIGNMENT: u64 = 256;

#[derive(Debug)]
struct BufferArrayStatus {
    new_item_count: usize,
    item_size: usize,
    staging_buffer_offset: usize,
    buffer: Option<RenderResource>,
}

pub struct UniformResourceProviderNew<T>
where
    T: AsUniforms + Send + Sync + 'static,
{
    _marker: PhantomData<T>,
    use_dynamic_uniforms: bool,
    is_instanceable: bool,
    // PERF: somehow remove this HashSet
    dynamic_uniform_buffer_status: Vec<Option<(String, BufferArrayStatus)>>,
    instance_buffer_status: Option<BufferArrayStatus>,
    query: Option<
        Query<
            (Read<T>, Write<Renderable>),
            EntityFilterTuple<
                And<(ComponentFilter<T>, ComponentFilter<Renderable>)>,
                And<(Passthrough, Passthrough)>,
                And<(Passthrough, Passthrough)>,
            >,
        >,
    >,
    query_finish: Option<
        Query<
            (Read<T>, Write<Renderable>),
            EntityFilterTuple<
                And<(ComponentFilter<T>, ComponentFilter<Renderable>)>,
                And<(Passthrough, Passthrough)>,
                And<(Passthrough, Passthrough)>,
            >,
        >,
    >,
    handle_query: Option<
        Query<
            (Read<Handle<T>>, Write<Renderable>),
            EntityFilterTuple<
                And<(ComponentFilter<Handle<T>>, ComponentFilter<Renderable>)>,
                And<(Passthrough, Passthrough)>,
                And<(Passthrough, Passthrough)>,
            >,
        >,
    >,
    handle_query_finish: Option<
        Query<
            (Read<Handle<T>>, Write<Renderable>),
            EntityFilterTuple<
                And<(ComponentFilter<Handle<T>>, ComponentFilter<Renderable>)>,
                And<(Passthrough, Passthrough)>,
                And<(Passthrough, Passthrough)>,
            >,
        >,
    >,
}

impl<T> UniformResourceProviderNew<T>
where
    T: AsUniforms + Send + Sync + 'static,
{
    pub fn new(use_dynamic_uniforms: bool) -> Self {
        let mut dynamic_uniform_buffer_status = Vec::new();
        let field_infos = T::get_field_infos();
        dynamic_uniform_buffer_status.resize_with(field_infos.len(), || None);
        let is_instanceable = field_infos.iter().find(|f| f.is_instanceable).is_some();
        UniformResourceProviderNew {
            dynamic_uniform_buffer_status,
            use_dynamic_uniforms,
            instance_buffer_status: None,
            is_instanceable,
            query: Some(<(Read<T>, Write<Renderable>)>::query()),
            query_finish: Some(<(Read<T>, Write<Renderable>)>::query()),
            handle_query: Some(<(Read<Handle<T>>, Write<Renderable>)>::query()),
            handle_query_finish: Some(<(Read<Handle<T>>, Write<Renderable>)>::query()),
            _marker: PhantomData,
        }
    }

    fn reset_buffer_array_status_counts(&mut self) {
        for buffer_status in self.dynamic_uniform_buffer_status.iter_mut() {
            if let Some((_name, buffer_status)) = buffer_status {
                buffer_status.new_item_count = 0;
            }
        }

        if let Some(ref mut buffer_status) = self.instance_buffer_status {
            buffer_status.new_item_count = 0;
        }
    }

    fn update_uniforms_info(&mut self, world: &mut World) {
        if !self.use_dynamic_uniforms {
            return;
        }

        let query = self.query.take().unwrap();
        for (uniforms, mut renderable) in query.iter_mut(world) {
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
            } else if self.use_dynamic_uniforms {
                self.increment_dynamic_uniform_counts(&uniforms);
            }

            Self::update_shader_defs(
                &uniforms,
                renderable.render_resource_assignments.as_mut().unwrap(),
            );
        }

        self.query = Some(query);
    }

    fn update_handles_info(&mut self, world: &mut World, resources: &Resources) {
        let handle_query = self.handle_query.take().unwrap();
        let assets = resources.get::<AssetStorage<T>>();
        let mut asset_batchers = resources.get_mut::<AssetBatchers>().unwrap();
        if let Some(assets) = assets {
            for (entity, (handle, mut renderable)) in handle_query.iter_entities_mut(world) {
                if !renderable.is_visible {
                    return;
                }

                if renderable.is_instanced {
                    if self.is_instanceable {
                        asset_batchers.set_entity_handle(entity, *handle);
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
                    Self::update_shader_defs(
                        uniforms,
                        renderable.render_resource_assignments.as_mut().unwrap(),
                    );
                    if self.use_dynamic_uniforms {
                        self.increment_dynamic_uniform_counts(&uniforms);
                    }
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
                item_size: f(),
                staging_buffer_offset: 0,
                buffer: None,
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

    fn increment_dynamic_uniform_counts(&mut self, uniforms: &T) {
        for (i, field_info) in T::get_field_infos().iter().enumerate() {
            if let Some(FieldBindType::Uniform { size }) =
                uniforms.get_field_bind_type(&field_info.name)
            {
                if let Some((ref _name, ref mut buffer_array_status)) =
                    self.dynamic_uniform_buffer_status[i]
                {
                    buffer_array_status.new_item_count += 1;
                } else {
                    self.dynamic_uniform_buffer_status[i] = Some((
                        field_info.uniform_name.to_string(),
                        BufferArrayStatus {
                            new_item_count: 1,
                            item_size: size,
                            staging_buffer_offset: 0,
                            buffer: None,
                        },
                    ))
                }
            }
        }
    }

    fn update_shader_defs(
        uniforms: &T,
        render_resource_assignments: &mut RenderResourceAssignments,
    ) {
        if let Some(shader_defs) = uniforms.get_shader_defs() {
            for shader_def in shader_defs {
                render_resource_assignments.shader_defs.insert(shader_def);
            }
        }
    }

    fn setup_uniform_resources(
        &mut self,
        uniforms: &T,
        renderer: &mut dyn Renderer,
        resources: &Resources,
        render_resource_assignments: &mut RenderResourceAssignments,
    ) {
        for field_info in T::get_field_infos() {
            let bind_type = uniforms.get_field_bind_type(&field_info.name);
            match bind_type {
                Some(FieldBindType::Uniform { size }) => {
                    if self.use_dynamic_uniforms {
                    } else {
                        let buffer_resource = match render_resource_assignments
                            .get(field_info.uniform_name)
                        {
                            Some(render_resource) => render_resource,
                            None => {
                                let resource = renderer.create_buffer(BufferInfo {
                                    size: size as u64,
                                    buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                                    ..Default::default()
                                });
                                render_resource_assignments.set(&field_info.uniform_name, resource);
                                resource
                            }
                        };

                        let (tmp_buffer, tmp_buffer_size) = if let Some(uniform_bytes) =
                            uniforms.get_uniform_bytes_ref(&field_info.uniform_name)
                        {
                            if size != uniform_bytes.len() {
                                panic!("The number of bytes produced for {} do not match the expected count. Actual: {}. Expected: {}.", field_info.uniform_name, uniform_bytes.len(), size);
                            }

                            (
                                renderer.create_buffer_mapped(
                                    BufferInfo {
                                        size: uniform_bytes.len() as u64,
                                        buffer_usage: BufferUsage::COPY_SRC,
                                        ..Default::default()
                                    },
                                    &mut |mapped| {
                                        mapped.copy_from_slice(uniform_bytes);
                                    },
                                ),
                                uniform_bytes.len(),
                            )
                        } else if let Some(uniform_bytes) =
                            uniforms.get_uniform_bytes(field_info.uniform_name)
                        {
                            if size != uniform_bytes.len() {
                                panic!("The number of bytes produced for {} do not match the expected count. Actual: {}. Expected: {}.", field_info.uniform_name, uniform_bytes.len(), size);
                            }

                            (
                                renderer.create_buffer_mapped(
                                    BufferInfo {
                                        size: uniform_bytes.len() as u64,
                                        buffer_usage: BufferUsage::COPY_SRC,
                                        ..Default::default()
                                    },
                                    &mut |mapped| {
                                        mapped.copy_from_slice(&uniform_bytes);
                                    },
                                ),
                                uniform_bytes.len(),
                            )
                        } else {
                            panic!(
                                "failed to get data from uniform: {}",
                                field_info.uniform_name
                            );
                        };

                        renderer.copy_buffer_to_buffer(
                            tmp_buffer,
                            0,
                            buffer_resource,
                            0,
                            tmp_buffer_size as u64,
                        );

                        renderer.remove_buffer(tmp_buffer);
                    }
                }
                Some(FieldBindType::Texture) => {
                    let texture_handle = uniforms
                        .get_uniform_texture(&field_info.texture_name)
                        .unwrap();
                    let (texture_resource, sampler_resource) = match renderer
                        .get_render_resources()
                        .get_texture_resource(texture_handle)
                    {
                        Some(texture_resource) => (
                            texture_resource,
                            renderer
                                .get_render_resources()
                                .get_texture_sampler_resource(texture_handle)
                                .unwrap(),
                        ),
                        None => {
                            let storage = resources.get::<AssetStorage<Texture>>().unwrap();
                            let texture = storage.get(&texture_handle).unwrap();

                            let texture_descriptor: TextureDescriptor = texture.into();
                            let texture_resource =
                                renderer.create_texture(&texture_descriptor, Some(&texture.data));

                            let sampler_descriptor: SamplerDescriptor = texture.into();
                            let sampler_resource = renderer.create_sampler(&sampler_descriptor);

                            let render_resources = renderer.get_render_resources_mut();
                            render_resources.set_texture_resource(texture_handle, texture_resource);
                            render_resources
                                .set_texture_sampler_resource(texture_handle, sampler_resource);
                            (texture_resource, sampler_resource)
                        }
                    };

                    render_resource_assignments.set(field_info.texture_name, texture_resource);
                    render_resource_assignments.set(field_info.sampler_name, sampler_resource);
                }
                None => {}
            }
        }
    }

    fn setup_uniforms_resources(
        &mut self,
        world: &mut World,
        resources: &Resources,
        renderer: &mut dyn Renderer,
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
                self.setup_uniform_resources(
                    &uniforms,
                    renderer,
                    resources,
                    renderable.render_resource_assignments.as_mut().unwrap(),
                )
            }
        }

        self.query_finish = Some(query_finish);
    }

    fn setup_handles_resources(
        &mut self,
        world: &mut World,
        resources: &Resources,
        renderer: &mut dyn Renderer,
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
                self.setup_uniform_resources(
                    &uniforms,
                    renderer,
                    resources,
                    renderable.render_resource_assignments.as_mut().unwrap(),
                )
            }

            self.handle_query_finish = Some(handle_query_finish);
        }
    }

    fn setup_batched_resources(
        &mut self,
        world: &mut World,
        resources: &Resources,
        renderer: &mut dyn Renderer,
    ) {
        // update batch resources. this needs to run in "finish_update" because batches aren't finalized across
        // all members of the batch until "UniformResourceProvider.update" has run for all members of the batch
        if let Some(asset_storage) = resources.get::<AssetStorage<T>>() {
            let mut asset_batchers = resources.get_mut::<AssetBatchers>().unwrap();
            let mut render_resource_assignments_provider = resources
                .get_mut::<RenderResourceAssignmentsProvider>()
                .unwrap();
            let handle_type = std::any::TypeId::of::<T>();
            for batch in asset_batchers.get_handle_batches_mut::<T>().unwrap() {
                let handle: Handle<T> = batch
                    .handles
                    .iter()
                    .find(|h| h.type_id == handle_type)
                    .map(|h| (*h).into())
                    .unwrap();

                let render_resource_assignments = batch
                    .render_resource_assignments
                    .get_or_insert_with(|| render_resource_assignments_provider.next());
                if let Some(uniforms) = asset_storage.get(&handle) {
                    self.setup_uniform_resources(
                        uniforms,
                        renderer,
                        resources,
                        render_resource_assignments,
                    );

                    Self::update_shader_defs(&uniforms, render_resource_assignments);
                }
            }
        }
    }

    fn setup_buffer_arrays(&mut self, renderer: &mut dyn Renderer) {
        for buffer_array_status in self.dynamic_uniform_buffer_status.iter_mut() {
            if let Some((name, buffer_array_status)) = buffer_array_status {
                println!("dynamic {} {:?}", name, buffer_array_status);
                Self::setup_buffer_array(buffer_array_status, renderer);
            }
        }

        if let Some(ref mut buffer_array_status) = self.instance_buffer_status {
            println!("instance {}", std::any::type_name::<T>());
            Self::setup_buffer_array(buffer_array_status, renderer);
        }
    }

    fn setup_buffer_array(
        buffer_array_status: &mut BufferArrayStatus,
        renderer: &mut dyn Renderer,
    ) {
        let new_capacity = if let Some(buffer) = buffer_array_status.buffer {
            if let Some(ResourceInfo::Buffer(BufferInfo {
                array_info: Some(array_info),
                ..
            })) = renderer.get_resource_info_mut(buffer)
            {
                if array_info.item_capacity < buffer_array_status.new_item_count as u64 {
                    Some(
                        buffer_array_status.new_item_count + buffer_array_status.new_item_count / 2,
                    )
                } else {
                    None
                }
            } else {
                Some(buffer_array_status.new_item_count)
            }
        } else {
            Some(buffer_array_status.new_item_count)
        };

        if let Some(new_capacity) = new_capacity {
            println!("creating buffer {}", new_capacity);
            let buffer = renderer.create_buffer(BufferInfo {
                array_info: Some(BufferArrayInfo {
                    item_capacity: new_capacity as u64,
                    item_count: buffer_array_status.new_item_count as u64,
                    item_size: buffer_array_status.item_size as u64,
                }),
                size: (buffer_array_status.item_size * new_capacity) as u64,
                buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                dynamic_uniform_info: Some(BufferDynamicUniformInfo::default()),
            });

            buffer_array_status.buffer = Some(buffer);
        }
    }

    fn initialize_vertex_buffer_descriptor(&self, render_graph: &mut RenderGraph) {
        let vertex_buffer_descriptor = T::get_vertex_buffer_descriptor();
        if let Some(vertex_buffer_descriptor) = vertex_buffer_descriptor {
            if let None = render_graph.get_vertex_buffer_descriptor(&vertex_buffer_descriptor.name)
            {
                render_graph.set_vertex_buffer_descriptor(vertex_buffer_descriptor.clone());
            }
        }
    }

    fn update_staging_buffer_offsets(&mut self) -> usize {
        let mut size = 0;
        for dynamic_buffer_array_status in self.dynamic_uniform_buffer_status.iter_mut() {
            if let Some((_name, ref mut buffer_array_status)) = dynamic_buffer_array_status {
                buffer_array_status.staging_buffer_offset = size;
                size += buffer_array_status.item_size * buffer_array_status.new_item_count;
            }
        }

        size
    }
}

impl<T> ResourceProvider for UniformResourceProviderNew<T>
where
    T: AsUniforms + Send + Sync + 'static,
{
    fn initialize(
        &mut self,
        renderer: &mut dyn Renderer,
        world: &mut World,
        resources: &Resources,
    ) {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        self.initialize_vertex_buffer_descriptor(&mut render_graph);
        self.update(renderer, world, resources);
    }

    fn update(&mut self, renderer: &mut dyn Renderer, world: &mut World, resources: &Resources) {
        self.reset_buffer_array_status_counts();
        self.update_uniforms_info(world);
        self.update_handles_info(world, resources);
    }

    fn finish_update(
        &mut self,
        renderer: &mut dyn Renderer,
        world: &mut World,
        resources: &Resources,
    ) {
        let staging_buffer_size = self.update_staging_buffer_offsets();
        // TODO: when setting batch shader_defs, add INSTANCING
        self.setup_buffer_arrays(renderer);
        self.setup_uniforms_resources(world, resources, renderer);
        self.setup_handles_resources(world, resources, renderer);
        self.setup_batched_resources(world, resources, renderer);
    }
}
