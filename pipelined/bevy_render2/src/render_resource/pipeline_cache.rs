use crate::{
    render_resource::{
        AsModuleDescriptorError, BindGroupLayout, BindGroupLayoutId, ProcessShaderError,
        RawFragmentState, RawRenderPipelineDescriptor, RawVertexState, RenderPipeline,
        RenderPipelineDescriptor, Shader, ShaderImport, ShaderProcessor,
    },
    renderer::RenderDevice,
    RenderWorld,
};
use bevy_app::EventReader;
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::system::{Res, ResMut};
use bevy_utils::{HashMap, HashSet};
use std::{collections::hash_map::Entry, hash::Hash, ops::Deref, sync::Arc};
use thiserror::Error;
use wgpu::{PipelineLayoutDescriptor, ShaderModule, VertexBufferLayout};

#[derive(Default)]
pub struct ShaderData {
    pipelines: HashSet<CachedPipelineId>,
    processed_shaders: HashMap<Vec<String>, Arc<ShaderModule>>,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct CachedPipelineId(usize);

impl CachedPipelineId {
    pub const INVALID: Self = CachedPipelineId(usize::MAX);
}

#[derive(Default)]
struct ShaderCache {
    data: HashMap<Handle<Shader>, ShaderData>,
    shaders: HashMap<Handle<Shader>, Shader>,
    import_path_shaders: HashMap<ShaderImport, Handle<Shader>>,
    processor: ShaderProcessor,
}

impl ShaderCache {
    fn get(
        &mut self,
        render_device: &RenderDevice,
        pipeline: CachedPipelineId,
        handle: &Handle<Shader>,
        shader_defs: &[String],
    ) -> Result<Arc<ShaderModule>, RenderPipelineError> {
        let shader = self
            .shaders
            .get(handle)
            .ok_or_else(|| RenderPipelineError::ShaderNotLoaded(handle.clone_weak()))?;
        let data = match self.data.entry(handle.clone_weak()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(ShaderData::default()),
        };
        data.pipelines.insert(pipeline);

        // PERF: this shader_defs clone isn't great. use raw_entry_mut when it stabilizes
        let module = match data.processed_shaders.entry(shader_defs.to_vec()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                for import in shader.imports() {
                    if !self.import_path_shaders.contains_key(import) {
                        return Err(RenderPipelineError::ShaderImportNotYetAvailable);
                    }
                }
                let processed = self.processor.process(
                    shader,
                    shader_defs,
                    &self.shaders,
                    &self.import_path_shaders,
                )?;
                let module_descriptor = processed.get_module_descriptor()?;
                entry.insert(Arc::new(
                    render_device.create_shader_module(&module_descriptor),
                ))
            }
        };
        Ok(module.clone())
    }

    fn clear(
        &mut self,
        handle: &Handle<Shader>,
    ) -> Option<impl Iterator<Item = CachedPipelineId> + '_> {
        let data = self.data.get_mut(handle)?;
        data.processed_shaders.clear();
        Some(data.pipelines.drain())
    }

    fn set_shader(&mut self, handle: &Handle<Shader>, shader: Shader) {
        if let Some(path) = shader.import_path() {
            self.import_path_shaders
                .insert(path.clone(), handle.clone_weak());
        }

        self.shaders.insert(handle.clone_weak(), shader);
    }

    fn remove(&mut self, handle: &Handle<Shader>) {
        if let Some(shader) = self.shaders.remove(handle) {
            if let Some(import_path) = shader.import_path() {
                self.import_path_shaders.remove(import_path);
            }
            self.data.remove(handle);
        }
    }
}

#[derive(Default)]
struct LayoutCache {
    layouts: HashMap<Vec<BindGroupLayoutId>, wgpu::PipelineLayout>,
}

impl LayoutCache {
    fn get(
        &mut self,
        render_device: &RenderDevice,
        bind_group_layouts: &[BindGroupLayout],
    ) -> &wgpu::PipelineLayout {
        let key = bind_group_layouts.iter().map(|l| l.id()).collect();
        self.layouts.entry(key).or_insert_with(|| {
            let bind_group_layouts = bind_group_layouts
                .iter()
                .map(|l| l.value())
                .collect::<Vec<_>>();
            render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
                bind_group_layouts: &bind_group_layouts,
                ..Default::default()
            })
        })
    }
}

pub struct RenderPipelineCache {
    layout_cache: LayoutCache,
    shader_cache: ShaderCache,
    device: RenderDevice,
    pipelines: Vec<CachedPipeline>,
    waiting_pipelines: Vec<CachedPipelineId>,
}

struct CachedPipeline {
    descriptor: RenderPipelineDescriptor,
    state: CachedPipelineState,
}

#[derive(Debug)]
pub enum CachedPipelineState {
    Queued,
    Ok(RenderPipeline),
    Err(RenderPipelineError),
}

impl CachedPipelineState {
    pub fn unwrap(&self) -> &RenderPipeline {
        match self {
            CachedPipelineState::Ok(pipeline) => pipeline,
            CachedPipelineState::Queued => {
                panic!("Pipeline has not been compiled yet. It is still in the 'Queued' state.")
            }
            CachedPipelineState::Err(err) => panic!("{}", err),
        }
    }
}

#[derive(Error, Debug)]
pub enum RenderPipelineError {
    #[error(
        "Pipeline cound not be compiled because the following shader is not loaded yet: {0:?}"
    )]
    ShaderNotLoaded(Handle<Shader>),
    #[error(transparent)]
    ProcessShaderError(#[from] ProcessShaderError),
    #[error(transparent)]
    AsModuleDescriptorError(#[from] AsModuleDescriptorError),
    #[error("Shader import not yet available.")]
    ShaderImportNotYetAvailable,
}

impl RenderPipelineCache {
    pub fn new(device: RenderDevice) -> Self {
        Self {
            device,
            layout_cache: Default::default(),
            shader_cache: Default::default(),
            waiting_pipelines: Default::default(),
            pipelines: Default::default(),
        }
    }

    #[inline]
    pub fn get_state(&self, id: CachedPipelineId) -> &CachedPipelineState {
        &self.pipelines[id.0].state
    }

    #[inline]
    pub fn get(&self, id: CachedPipelineId) -> Option<&RenderPipeline> {
        if let CachedPipelineState::Ok(pipeline) = &self.pipelines[id.0].state {
            Some(pipeline)
        } else {
            None
        }
    }

    pub fn queue(&mut self, descriptor: RenderPipelineDescriptor) -> CachedPipelineId {
        let id = CachedPipelineId(self.pipelines.len());
        self.pipelines.push(CachedPipeline {
            descriptor,
            state: CachedPipelineState::Queued,
        });
        self.waiting_pipelines.push(id);
        id
    }

    fn set_shader(&mut self, handle: &Handle<Shader>, shader: &Shader) {
        if let Some(cached_pipelines) = self.shader_cache.clear(handle) {
            for cached_pipeline in cached_pipelines {
                self.pipelines[cached_pipeline.0].state = CachedPipelineState::Queued;
                self.waiting_pipelines.push(cached_pipeline);
            }
        }

        self.shader_cache.set_shader(handle, shader.clone());
    }

    fn remove_shader(&mut self, shader: &Handle<Shader>) {
        if let Some(cached_pipelines) = self.shader_cache.clear(shader) {
            for cached_pipeline in cached_pipelines {
                self.pipelines[cached_pipeline.0].state = CachedPipelineState::Queued;
                self.waiting_pipelines.push(cached_pipeline);
            }
        }

        self.shader_cache.remove(shader);
    }

    pub fn process_queue(&mut self) {
        let pipelines = std::mem::take(&mut self.waiting_pipelines);
        for id in pipelines {
            let state = &mut self.pipelines[id.0];
            match &state.state {
                CachedPipelineState::Ok(_) => continue,
                CachedPipelineState::Queued => {}
                CachedPipelineState::Err(err) => {
                    match err {
                        RenderPipelineError::ShaderNotLoaded(_)
                        | RenderPipelineError::ShaderImportNotYetAvailable => { /* retry */ }
                        RenderPipelineError::ProcessShaderError(_)
                        | RenderPipelineError::AsModuleDescriptorError(_) => {
                            // shader could not be processed ... retrying won't help
                            continue;
                        }
                    }
                }
            }

            let descriptor = &state.descriptor;
            let vertex_module = match self.shader_cache.get(
                &self.device,
                id,
                &descriptor.vertex.shader,
                &descriptor.vertex.shader_defs,
            ) {
                Ok(module) => module,
                Err(err) => {
                    state.state = CachedPipelineState::Err(err);
                    self.waiting_pipelines.push(id);
                    continue;
                }
            };

            let fragment_data = if let Some(fragment) = &descriptor.fragment {
                let fragment_module = match self.shader_cache.get(
                    &self.device,
                    id,
                    &fragment.shader,
                    &fragment.shader_defs,
                ) {
                    Ok(module) => module,
                    Err(err) => {
                        state.state = CachedPipelineState::Err(err);
                        self.waiting_pipelines.push(id);
                        continue;
                    }
                };
                Some((
                    fragment_module,
                    fragment.entry_point.deref(),
                    &fragment.targets,
                ))
            } else {
                None
            };

            let vertex_buffer_layouts = descriptor
                .vertex
                .buffers
                .iter()
                .map(|layout| VertexBufferLayout {
                    array_stride: layout.array_stride,
                    attributes: &layout.attributes,
                    step_mode: layout.step_mode,
                })
                .collect::<Vec<_>>();

            let layout = if let Some(layout) = &descriptor.layout {
                Some(self.layout_cache.get(&self.device, layout))
            } else {
                None
            };

            let descriptor = RawRenderPipelineDescriptor {
                depth_stencil: descriptor.depth_stencil.clone(),
                label: descriptor.label.as_deref(),
                layout,
                multisample: descriptor.multisample,
                primitive: descriptor.primitive,
                vertex: RawVertexState {
                    buffers: &vertex_buffer_layouts,
                    entry_point: descriptor.vertex.entry_point.deref(),
                    module: &vertex_module,
                },
                fragment: fragment_data
                    .as_ref()
                    .map(|(module, entry_point, targets)| RawFragmentState {
                        entry_point,
                        module,
                        targets,
                    }),
            };

            let pipeline = self.device.create_render_pipeline(&descriptor);
            state.state = CachedPipelineState::Ok(pipeline);
        }
    }

    pub(crate) fn process_pipeline_queue_system(mut cache: ResMut<Self>) {
        cache.process_queue();
    }

    pub(crate) fn extract_shaders(
        mut world: ResMut<RenderWorld>,
        shaders: Res<Assets<Shader>>,
        mut events: EventReader<AssetEvent<Shader>>,
    ) {
        let mut cache = world.get_resource_mut::<Self>().unwrap();
        for event in events.iter() {
            match event {
                AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                    if let Some(shader) = shaders.get(handle) {
                        cache.set_shader(handle, shader);
                    }
                }
                AssetEvent::Removed { handle } => cache.remove_shader(handle),
            }
        }
    }
}
