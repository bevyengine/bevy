use crate::render::render_graph::{
    AsUniforms, BindType, DynamicUniformBufferInfo, Renderable, Renderer, ResourceProvider,
};
use legion::prelude::*;
use std::marker::PhantomData;

pub struct UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync,
{
    _marker: PhantomData<T>,
    uniform_buffer_info_names: Vec<String>,
}

impl<T> UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync,
{
    pub fn new() -> Self {
        UniformResourceProvider {
            uniform_buffer_info_names: Vec::new(),
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
        for name in self.uniform_buffer_info_names.iter() {
            renderer
                .get_dynamic_uniform_buffer_info_mut(name)
                .unwrap()
                .count = 0;
        }

        let mut sizes = Vec::new();
        let mut counts = Vec::new();
        for (uniforms, _renderable) in query.iter(world) {
            let uniform_layouts = uniforms.get_uniform_layouts();
            for (i, uniform_info) in uniforms
                .get_uniform_infos()
                .iter()
                .filter(|u| {
                    if let BindType::Uniform { .. } = u.bind_type {
                        true
                    } else {
                        false
                    }
                })
                .enumerate()
            {
                // only add the first time a uniform info is processed
                if self.uniform_buffer_info_names.len() <= i {
                    let uniform_layout = uniform_layouts[i];
                    // TODO: size is 0 right now because uniform layout isn't populated
                    // also size isn't even being used right now?
                    let size = uniform_layout
                        .iter()
                        .map(|u| u.get_size())
                        .fold(0, |total, current| total + current);
                    sizes.push(size);

                    self.uniform_buffer_info_names
                        .push(uniform_info.name.to_string());
                }

                if counts.len() <= i {
                    counts.push(0);
                }

                counts[i] += 1;
            }
        }

        // create and update uniform buffer info. this is separate from the last block to avoid
        // the expense of hashing for large numbers of entities
        for (i, name) in self.uniform_buffer_info_names.iter().enumerate() {
            if let None = renderer.get_dynamic_uniform_buffer_info(name) {
                let mut info = DynamicUniformBufferInfo::new();
                info.size = sizes[i];
                renderer.add_dynamic_uniform_buffer_info(name, info);
            }

            let info = renderer.get_dynamic_uniform_buffer_info_mut(name).unwrap();
            info.count = counts[i];
        }

        // allocate uniform buffers
        for name in self.uniform_buffer_info_names.iter() {
            if let Some(_) = renderer.get_resource_info(name) {
                continue;
            }

            let info = renderer.get_dynamic_uniform_buffer_info_mut(name).unwrap();

            // allocate enough space for twice as many entities as there are currently;
            info.capacity = info.count * 2;
            let size = wgpu::BIND_BUFFER_ALIGNMENT * info.capacity;
            renderer.create_buffer(
                name,
                size,
                wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM,
            );
        }

        // copy entity uniform data to buffers
        for name in self.uniform_buffer_info_names.iter() {
            let size = {
                let info = renderer.get_dynamic_uniform_buffer_info(name).unwrap();
                wgpu::BIND_BUFFER_ALIGNMENT * info.count
            };

            let alignment = wgpu::BIND_BUFFER_ALIGNMENT as usize;
            let mut offset = 0usize;
            let info = renderer.get_dynamic_uniform_buffer_info_mut(name).unwrap();
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
            renderer.create_buffer_mapped(
                "tmp_uniform_mapped",
                size as usize,
                wgpu::BufferUsage::COPY_SRC,
                &mut |mapped| {
                    let alignment = wgpu::BIND_BUFFER_ALIGNMENT as usize;
                    let mut offset = 0usize;
                    for (uniforms, _renderable) in query.iter(world) {
                        // TODO: check if index has changed. if it has, then entity should be updated
                        // TODO: only mem-map entities if their data has changed
                        // TODO: try getting bytes ref first
                        if let Some(uniform_bytes) = uniforms.get_uniform_bytes(name) {
                            mapped[offset..(offset + uniform_bytes.len())]
                                .copy_from_slice(uniform_bytes.as_slice());
                            offset += alignment;
                        }
                    }
                },
            );

            renderer.copy_buffer_to_buffer("tmp_uniform_mapped", 0, name, 0, size);
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
