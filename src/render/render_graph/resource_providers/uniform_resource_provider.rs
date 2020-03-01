use crate::{
    asset::{AssetStorage, Texture},
    render::render_graph::{
        render_resource::RenderResource, AsUniforms, BindType, DynamicUniformBufferInfo,
        Renderable, Renderer, ResourceProvider, TextureDescriptor, UniformInfoIter, SamplerDescriptor,
    },
};
use legion::prelude::*;
use std::{marker::PhantomData, ops::Deref};

pub struct UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync,
{
    _marker: PhantomData<T>,
    uniform_buffer_info_resources: Vec<(String, Option<RenderResource>)>,
}

impl<T> UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync,
{
    pub fn new() -> Self {
        UniformResourceProvider {
            uniform_buffer_info_resources: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<T> ResourceProvider for UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync + 'static,
{
    fn initialize(&mut self, renderer: &mut dyn Renderer, world: &mut World) {
        self.update(renderer, world);
    }

    fn update(&mut self, renderer: &mut dyn Renderer, world: &mut World) {
        let query = <(Read<T>, Read<Renderable>)>::query();
        // TODO: this breaks down in multiple ways:
        // (SOLVED 1) resource_info will be set after the first run so this won't update.
        // (2) if we create new buffers, the old bind groups will be invalid

        // reset all uniform buffer info counts
        for (_name, resource) in self.uniform_buffer_info_resources.iter() {
            renderer
                .get_dynamic_uniform_buffer_info_mut(resource.unwrap())
                .unwrap()
                .count = 0;
        }

        let mut counts = Vec::new();
        for (entity, (uniforms, _renderable)) in query.iter_entities(world) {
            let mut uniform_index = 0;
            let field_uniform_names = uniforms.get_field_uniform_names();
            for uniform_info in UniformInfoIter::new(field_uniform_names, uniforms.deref()) {
                match uniform_info.bind_type {
                    BindType::Uniform { .. } => {
                        // only add the first time a uniform info is processed
                        if self.uniform_buffer_info_resources.len() <= uniform_index {
                            self.uniform_buffer_info_resources
                                .push((uniform_info.name.to_string(), None));
                        }

                        if counts.len() <= uniform_index {
                            counts.push(0);
                        }

                        counts[uniform_index] += 1;
                        uniform_index += 1;
                    }
                    BindType::SampledTexture { .. } => {
                        let texture_handle =
                            uniforms.get_uniform_texture(&uniform_info.name).unwrap();
                        let storage = world.resources.get::<AssetStorage<Texture>>().unwrap();
                        let texture = storage.get(&texture_handle).unwrap();
                        let resource = match renderer.get_render_resources().get_texture_resource(texture_handle) {
                            Some(resource) => resource,
                            None => {
                                let descriptor: TextureDescriptor = texture.into();
                                let resource =
                                    renderer.create_texture(&descriptor, Some(&texture.data));
                                renderer.get_render_resources_mut().set_texture_resource(texture_handle, resource);
                                resource
                            }
                        };
                        
                        renderer.assign_entity_uniform_resource(*entity, uniform_info.name, resource);
                    }
                    BindType::Sampler { .. } => {
                        let texture_handle =
                            uniforms.get_uniform_texture(&uniform_info.name).unwrap();
                        let storage = world.resources.get::<AssetStorage<Texture>>().unwrap();
                        let texture = storage.get(&texture_handle).unwrap();
                        let resource = match renderer.get_render_resources().get_texture_sampler_resource(texture_handle) {
                            Some(resource) => resource,
                            None => {
                                let descriptor: SamplerDescriptor = texture.into();
                                let resource =
                                    renderer.create_sampler(&descriptor);
                                renderer.get_render_resources_mut().set_texture_sampler_resource(texture_handle, resource);
                                resource
                            }
                        };

                        renderer.assign_entity_uniform_resource(*entity, uniform_info.name, resource);
                    }
                    _ => panic!(
                        "encountered unsupported bind_type {:?}",
                        uniform_info.bind_type
                    ),
                }
            }
        }

        // allocate uniform buffers
        for (i, (name, resource)) in self.uniform_buffer_info_resources.iter_mut().enumerate() {
            if let Some(resource) = resource {
                let mut info = renderer
                    .get_dynamic_uniform_buffer_info_mut(*resource)
                    .unwrap();
                info.count = counts[i];
                continue;
            }

            // allocate enough space for twice as many entities as there are currently;
            let capacity = counts[i] * 2;
            let size = wgpu::BIND_BUFFER_ALIGNMENT * capacity;
            let created_resource = renderer.create_buffer(
                size,
                wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM,
            );

            let mut info = DynamicUniformBufferInfo::new();
            info.count = counts[i];
            info.capacity = capacity;
            renderer.add_dynamic_uniform_buffer_info(created_resource, info);
            *resource = Some(created_resource);
            renderer.get_render_resources_mut().set_named_resource(name, created_resource);
        }

        // copy entity uniform data to buffers
        for (name, resource) in self.uniform_buffer_info_resources.iter() {
            let resource = resource.unwrap();
            let size = {
                let info = renderer.get_dynamic_uniform_buffer_info(resource).unwrap();
                wgpu::BIND_BUFFER_ALIGNMENT * info.count
            };

            let alignment = wgpu::BIND_BUFFER_ALIGNMENT as usize;
            let mut offset = 0usize;
            let info = renderer
                .get_dynamic_uniform_buffer_info_mut(resource)
                .unwrap();
            for (i, (entity, _)) in query.iter_entities(world).enumerate() {
                // TODO: check if index has changed. if it has, then entity should be updated
                // TODO: only mem-map entities if their data has changed
                // PERF: These hashmap inserts are pretty expensive (10 fps for 10000 entities)
                info.offsets.insert(entity, offset as u64);
                info.indices.insert(i, entity);
                // TODO: try getting ref first
                offset += alignment;
            }

            // let mut data = vec![Default::default(); size as usize];
            let mapped_buffer_resource = renderer.create_buffer_mapped(
                size as usize,
                wgpu::BufferUsage::COPY_SRC,
                &mut |mapped| {
                    let alignment = wgpu::BIND_BUFFER_ALIGNMENT as usize;
                    let mut offset = 0usize;
                    for (uniforms, _renderable) in query.iter(world) {
                        // TODO: check if index has changed. if it has, then entity should be updated
                        // TODO: only mem-map entities if their data has changed
                        // TODO: try getting bytes ref first
                        if let Some(uniform_bytes) = uniforms.get_uniform_bytes(&name) {
                            mapped[offset..(offset + uniform_bytes.len())]
                                .copy_from_slice(uniform_bytes.as_slice());
                            offset += alignment;
                        }
                    }
                },
            );

            renderer.copy_buffer_to_buffer(mapped_buffer_resource, 0, resource, 0, size);

            // TODO: uncomment this to free resource?
            renderer.remove_buffer(mapped_buffer_resource);
        }

        // update shader assignments based on current macro defs
        for (uniforms, mut renderable) in <(Read<T>, Write<Renderable>)>::query().iter_mut(world) {
            if let Some(shader_defs) = uniforms.get_shader_defs() {
                for shader_def in shader_defs {
                    renderable.shader_defs.insert(shader_def);
                }
            }
        }
    }

    fn resize(
        &mut self,
        _renderer: &mut dyn Renderer,
        _world: &mut World,
        _width: u32,
        _height: u32,
    ) {
    }
}
