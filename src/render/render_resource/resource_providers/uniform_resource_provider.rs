use crate::{
    asset::{AssetStorage, Handle},
    render::{
        pipeline::BindType,
        render_resource::{
            AssetBatchers, BufferArrayInfo, BufferInfo, BufferUsage,
            EntityRenderResourceAssignments, RenderResource, RenderResourceAssignments,
            RenderResourceAssignmentsProvider, ResourceInfo, ResourceProvider, BufferDynamicUniformInfo,
        },
        renderer::Renderer,
        shader::{AsUniforms, UniformInfoIter},
        texture::{SamplerDescriptor, Texture, TextureDescriptor},
        Renderable,
    },
};
use legion::{filter::*, prelude::*};
use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::Deref,
};
pub const BIND_BUFFER_ALIGNMENT: u64 = 256;

pub struct UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync + 'static,
{
    _marker: PhantomData<T>,
    // PERF: somehow remove this HashSet
    uniform_buffer_info_resources:
        HashMap<String, (Option<RenderResource>, usize, HashSet<Entity>)>,
    asset_resources: HashMap<Handle<T>, HashMap<String, RenderResource>>,
    resource_query: Query<
        (Read<T>, Read<Renderable>),
        EntityFilterTuple<
            And<(ComponentFilter<T>, ComponentFilter<Renderable>)>,
            And<(Passthrough, Passthrough)>,
            And<(Passthrough, Passthrough)>,
        >,
    >,
    handle_query: Option<
        Query<
            (Read<Handle<T>>, Read<Renderable>),
            EntityFilterTuple<
                And<(ComponentFilter<Handle<T>>, ComponentFilter<Renderable>)>,
                And<(Passthrough, Passthrough)>,
                And<(Passthrough, Passthrough)>,
            >,
        >,
    >,
}

impl<T> UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync + 'static,
{
    pub fn new() -> Self {
        UniformResourceProvider {
            uniform_buffer_info_resources: Default::default(),
            asset_resources: Default::default(),
            _marker: PhantomData,
            resource_query: <(Read<T>, Read<Renderable>)>::query(),
            handle_query: Some(<(Read<Handle<T>>, Read<Renderable>)>::query()),
        }
    }

    fn update_asset_uniforms(
        &mut self,
        renderer: &mut dyn Renderer,
        world: &World,
        resources: &Resources,
    ) {
        let handle_query = self.handle_query.take().unwrap();
        let mut asset_batchers = resources.get_mut::<AssetBatchers>().unwrap();
        let mut entity_render_resource_assignments = resources
            .get_mut::<EntityRenderResourceAssignments>()
            .unwrap();
        let mut render_resource_assignments_provider = resources
            .get_mut::<RenderResourceAssignmentsProvider>()
            .unwrap();
        // TODO: only update handle values when Asset value has changed
        if let Some(asset_storage) = resources.get::<AssetStorage<T>>() {
            for (entity, (handle, renderable)) in handle_query.iter_entities(world) {
                if renderable.is_instanced {
                    asset_batchers.set_entity_handle(entity, *handle);
                } else {
                    let render_resource_assignments = if let Some(assignments) =
                        entity_render_resource_assignments.get_mut(entity)
                    {
                        assignments
                    } else {
                        entity_render_resource_assignments
                            .set(entity, render_resource_assignments_provider.next());
                        entity_render_resource_assignments.get_mut(entity).unwrap()
                    };
                    if let Some(uniforms) = asset_storage.get(&handle) {
                        self.setup_entity_uniform_resources(
                            entity,
                            uniforms,
                            renderer,
                            resources,
                            render_resource_assignments,
                            true,
                            Some(*handle),
                        )
                    }
                }
            }
        }

        self.handle_query = Some(handle_query);
    }

    fn setup_entity_uniform_resources(
        &mut self,
        entity: Entity,
        uniforms: &T,
        renderer: &mut dyn Renderer,
        resources: &Resources,
        render_resource_assignments: &mut RenderResourceAssignments,
        dynamic_unforms: bool,
        asset_handle: Option<Handle<T>>,
    ) {
        let field_infos = uniforms.get_field_infos();
        for uniform_info in UniformInfoIter::new(field_infos, uniforms.deref()) {
            match uniform_info.bind_type {
                BindType::Uniform { .. } => {
                    if dynamic_unforms {
                        if let None = self.uniform_buffer_info_resources.get(uniform_info.name) {
                            self.uniform_buffer_info_resources
                                .insert(uniform_info.name.to_string(), (None, 0, HashSet::new()));
                        }

                        let (_resource, counts, entities) = self
                            .uniform_buffer_info_resources
                            .get_mut(uniform_info.name)
                            .unwrap();
                        entities.insert(entity);
                        *counts += 1;
                    } else {
                        let handle = asset_handle.expect(
                            "non-dynamic uniform currently only works with Handle<T> types",
                        );
                        if let None = self.asset_resources.get(&handle) {
                            self.asset_resources.insert(handle, HashMap::new());
                        }

                        let resources = self.asset_resources.get_mut(&handle).unwrap();

                        let render_resource = match resources.get(uniform_info.name) {
                            Some(render_resource) => *render_resource,
                            None => {
                                // let size = uniform_info.bind_type.get_uniform_size().unwrap();
                                // TODO: get actual size here
                                let size = BIND_BUFFER_ALIGNMENT;
                                let resource = renderer.create_buffer(BufferInfo {
                                    size,
                                    buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                                    ..Default::default()
                                });
                                resources.insert(uniform_info.name.to_string(), resource);
                                resource
                            }
                        };

                        render_resource_assignments.set(uniform_info.name, render_resource);

                        let (tmp_buffer, tmp_buffer_size) = if let Some(uniform_bytes) =
                            uniforms.get_uniform_bytes_ref(uniform_info.name)
                        {
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
                            uniforms.get_uniform_bytes(uniform_info.name)
                        {
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
                            panic!("failed to get data from uniform: {}", uniform_info.name);
                        };

                        renderer.copy_buffer_to_buffer(
                            tmp_buffer,
                            0,
                            render_resource,
                            0,
                            tmp_buffer_size as u64,
                        );

                        renderer.remove_buffer(tmp_buffer);
                    }
                }
                BindType::SampledTexture { .. } => {
                    let texture_handle = uniforms.get_uniform_texture(&uniform_info.name).unwrap();
                    let resource = match renderer
                        .get_render_resources()
                        .get_texture_resource(texture_handle)
                    {
                        Some(resource) => resource,
                        None => {
                            let storage = resources.get::<AssetStorage<Texture>>().unwrap();
                            let texture = storage.get(&texture_handle).unwrap();
                            let descriptor: TextureDescriptor = texture.into();
                            let resource =
                                renderer.create_texture(&descriptor, Some(&texture.data));
                            renderer
                                .get_render_resources_mut()
                                .set_texture_resource(texture_handle, resource);
                            resource
                        }
                    };

                    render_resource_assignments.set(uniform_info.name, resource);
                }
                BindType::Sampler { .. } => {
                    let texture_handle = uniforms.get_uniform_texture(&uniform_info.name).unwrap();
                    let resource = match renderer
                        .get_render_resources()
                        .get_texture_sampler_resource(texture_handle)
                    {
                        Some(resource) => resource,
                        None => {
                            let storage = resources.get::<AssetStorage<Texture>>().unwrap();
                            let texture = storage.get(&texture_handle).unwrap();
                            let descriptor: SamplerDescriptor = texture.into();
                            let resource = renderer.create_sampler(&descriptor);
                            renderer
                                .get_render_resources_mut()
                                .set_texture_sampler_resource(texture_handle, resource);
                            resource
                        }
                    };

                    render_resource_assignments.set(uniform_info.name, resource);
                }
                _ => panic!(
                    "encountered unsupported bind_type {:?}",
                    uniform_info.bind_type
                ),
            }
        }
    }

    fn setup_dynamic_uniform_buffers(
        &mut self,
        renderer: &mut dyn Renderer,
        world: &World,
        resources: &Resources,
    ) {
        let entity_render_resource_assignments = resources
            .get::<EntityRenderResourceAssignments>()
            .unwrap();
        // allocate uniform buffers
        for (name, (resource, count, _entities)) in self.uniform_buffer_info_resources.iter_mut() {
            let count = *count as u64;
            if let Some(resource) = resource {
                if let Some(ResourceInfo::Buffer(BufferInfo {
                    array_info: Some(array_info),
                    ..
                })) = renderer.get_resource_info_mut(*resource)
                {
                    array_info.item_count = count;
                    continue;
                }
            }

            // allocate enough space for twice as many entities as there are currently;
            let capacity = count * 2;
            let size = BIND_BUFFER_ALIGNMENT * capacity;
            let created_resource = renderer.create_buffer(BufferInfo {
                size,
                buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                array_info: Some(BufferArrayInfo {
                    item_capacity: capacity,
                    item_count: count,
                    item_size: BIND_BUFFER_ALIGNMENT,
                }),
                dynamic_uniform_info: Some(BufferDynamicUniformInfo::default()),
                ..Default::default()
            });

            *resource = Some(created_resource);
            renderer
                .get_render_resources_mut()
                .set_named_resource(name, created_resource);
        }

        // copy entity uniform data to buffers
        // PERF: consider iter_chunks here and calling get_bytes() on each chunk?
        for (name, (resource, count, entities)) in self.uniform_buffer_info_resources.iter() {
            let resource = resource.unwrap();
            let resource_info = renderer.get_resource_info_mut(resource);
            if let Some(ResourceInfo::Buffer(BufferInfo {
                dynamic_uniform_info: Some(dynamic_uniform_info),
                ..
            })) = resource_info
            {
                let size = BIND_BUFFER_ALIGNMENT * *count as u64;
                let alignment = BIND_BUFFER_ALIGNMENT as usize;
                let mut offset = 0usize;
                // TODO: check if index has changed. if it has, then entity should be updated
                // TODO: only mem-map entities if their data has changed
                // PERF: These hashmap inserts are pretty expensive (10 fps for 10000 entities)
                for (entity, (_, renderable)) in self.resource_query.iter_entities(world) {
                    if renderable.is_instanced || !entities.contains(&entity) {
                        continue;
                    }

                    if let Some(render_resource) = entity_render_resource_assignments.get(entity) {
                        dynamic_uniform_info.offsets.insert(render_resource.get_id(), offset as u32);
                    }

                    offset += alignment;
                }

                for (entity, (_, renderable)) in
                    self.handle_query.as_ref().unwrap().iter_entities(world)
                {
                    if renderable.is_instanced {
                        continue;
                    }

                    if let Some(render_resource) = entity_render_resource_assignments.get(entity) {
                        dynamic_uniform_info.offsets.insert(render_resource.get_id(), offset as u32);
                    }

                    offset += alignment;
                }

                // TODO: check if index has changed. if it has, then entity should be updated
                // TODO: only mem-map entities if their data has changed
                let mapped_buffer_resource = renderer.create_buffer_mapped(
                    BufferInfo {
                        size,
                        buffer_usage: BufferUsage::COPY_SRC,
                        ..Default::default()
                    },
                    &mut |mapped| {
                        let alignment = BIND_BUFFER_ALIGNMENT as usize;
                        let mut offset = 0usize;
                        for (entity, (uniforms, renderable)) in
                            self.resource_query.iter_entities(world)
                        {
                            if renderable.is_instanced || !entities.contains(&entity) {
                                continue;
                            }
                            if let Some(uniform_bytes) = uniforms.get_uniform_bytes_ref(&name) {
                                mapped[offset..(offset + uniform_bytes.len())]
                                    .copy_from_slice(uniform_bytes);
                                offset += alignment;
                            } else if let Some(uniform_bytes) = uniforms.get_uniform_bytes(&name) {
                                mapped[offset..(offset + uniform_bytes.len())]
                                    .copy_from_slice(uniform_bytes.as_slice());
                                offset += alignment;
                            }
                        }

                        if let Some(asset_storage) = resources.get::<AssetStorage<T>>() {
                            for (entity, (handle, renderable)) in
                                self.handle_query.as_ref().unwrap().iter_entities(world)
                            {
                                if renderable.is_instanced || !entities.contains(&entity) {
                                    continue;
                                }

                                let uniforms = asset_storage.get(&handle).unwrap();
                                if let Some(uniform_bytes) = uniforms.get_uniform_bytes_ref(&name) {
                                    mapped[offset..(offset + uniform_bytes.len())]
                                        .copy_from_slice(uniform_bytes);
                                    offset += alignment;
                                } else if let Some(uniform_bytes) =
                                    uniforms.get_uniform_bytes(&name)
                                {
                                    mapped[offset..(offset + uniform_bytes.len())]
                                        .copy_from_slice(uniform_bytes.as_slice());
                                    offset += alignment;
                                }
                            }
                        }
                    },
                );

                renderer.copy_buffer_to_buffer(mapped_buffer_resource, 0, resource, 0, size);
                renderer.remove_buffer(mapped_buffer_resource);
            }
        }
    }

    fn initialize_vertex_buffer_descriptor(&self, renderer: &mut dyn Renderer) {
        let vertex_buffer_descriptor = T::get_vertex_buffer_descriptor();
        if let Some(vertex_buffer_descriptor) = vertex_buffer_descriptor {
            if let None = renderer.get_vertex_buffer_descriptor(&vertex_buffer_descriptor.name) {
                renderer.set_vertex_buffer_descriptor(vertex_buffer_descriptor.clone());
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
        renderer: &mut dyn Renderer,
        world: &mut World,
        resources: &Resources,
    ) {
        self.update(renderer, world, resources);
    }

    fn update(&mut self, renderer: &mut dyn Renderer, world: &mut World, resources: &Resources) {
        self.initialize_vertex_buffer_descriptor(renderer);

        // TODO: this breaks down in multiple ways:
        // (SOLVED 1) resource_info will be set after the first run so this won't update.
        // (2) if we create new buffers, the old bind groups will be invalid

        // reset all uniform buffer info counts
        let query = <(Read<T>, Read<Renderable>)>::query();
        for (_name, (resource, count, _entities)) in self.uniform_buffer_info_resources.iter_mut() {
            if let Some(ResourceInfo::Buffer(BufferInfo {
                array_info: Some(array_info),
                ..
            })) = renderer.get_resource_info_mut(resource.unwrap())
            {
                array_info.item_count = 0;
            }

            *count = 0;
        }

        self.update_asset_uniforms(renderer, world, resources);

        let mut entity_render_resource_assignments = resources
            .get_mut::<EntityRenderResourceAssignments>()
            .unwrap();
        let mut render_resource_assignments_provider = resources
            .get_mut::<RenderResourceAssignmentsProvider>()
            .unwrap();
        for (entity, (uniforms, renderable)) in query.iter_entities(world) {
            if renderable.is_instanced {
                continue;
            }

            let render_resource_assignments =
                if let Some(assignments) = entity_render_resource_assignments.get_mut(entity) {
                    assignments
                } else {
                    entity_render_resource_assignments
                        .set(entity, render_resource_assignments_provider.next());
                    entity_render_resource_assignments.get_mut(entity).unwrap()
                };
            self.setup_entity_uniform_resources(
                entity,
                &uniforms,
                renderer,
                resources,
                render_resource_assignments,
                true,
                None,
            );
        }

        self.setup_dynamic_uniform_buffers(renderer, world, resources);

        // update shader assignments based on current macro defs
        for (uniforms, mut renderable) in <(Read<T>, Write<Renderable>)>::query().iter_mut(world) {
            if let Some(shader_defs) = uniforms.get_shader_defs() {
                for shader_def in shader_defs {
                    renderable.shader_defs.insert(shader_def);
                }
            }
        }

        if let Some(asset_storage) = resources.get::<AssetStorage<T>>() {
            for (handle, mut renderable) in
                <(Read<Handle<T>>, Write<Renderable>)>::query().iter_mut(world)
            {
                let uniforms = asset_storage.get(&handle).unwrap();
                if let Some(shader_defs) = uniforms.get_shader_defs() {
                    for shader_def in shader_defs {
                        renderable.shader_defs.insert(shader_def);
                    }
                }
            }
        }
    }
}
