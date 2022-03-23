use crate::{
    render_resource::{
        AsModuleDescriptorError, BindGroupLayout, BindGroupLayoutId, ProcessShaderError,
        RawFragmentState, RawRenderPipelineDescriptor, RawVertexState, RenderPipeline,
        RenderPipelineDescriptor, Shader, ShaderImport, ShaderProcessor, ShaderReflectError,
    },
    renderer::RenderDevice,
    RenderWorld,
};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::event::EventReader;
use bevy_ecs::system::{Res, ResMut};
use bevy_utils::{tracing::error, Entry, HashMap, HashSet};
use std::{hash::Hash, ops::Deref, sync::Arc};
use thiserror::Error;
use wgpu::{PipelineLayoutDescriptor, ShaderModule, VertexBufferLayout as RawVertexBufferLayout};

use super::ProcessedShader;

#[derive(Default)]
pub struct ShaderData {
    pipelines: HashSet<CachedPipelineId>,
    processed_shaders: HashMap<Vec<String>, Arc<ShaderModule>>,
    resolved_imports: HashMap<ShaderImport, Handle<Shader>>,
    dependents: HashSet<Handle<Shader>>,
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
    waiting_on_import: HashMap<ShaderImport, Vec<Handle<Shader>>>,
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
        let data = self.data.entry(handle.clone_weak()).or_default();
        let n_asset_imports = shader
            .imports()
            .filter(|import| matches!(import, ShaderImport::AssetPath(_)))
            .count();
        let n_resolved_asset_imports = data
            .resolved_imports
            .keys()
            .filter(|import| matches!(import, ShaderImport::AssetPath(_)))
            .count();
        if n_asset_imports != n_resolved_asset_imports {
            return Err(RenderPipelineError::ShaderImportNotYetAvailable);
        }

        data.pipelines.insert(pipeline);

        // PERF: this shader_defs clone isn't great. use raw_entry_mut when it stabilizes
        let module = match data.processed_shaders.entry(shader_defs.to_vec()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let processed = self.processor.process(
                    shader,
                    shader_defs,
                    &self.shaders,
                    &self.import_path_shaders,
                )?;
                let module_descriptor = match processed.get_module_descriptor() {
                    Ok(module_descriptor) => module_descriptor,
                    Err(err) => {
                        return Err(RenderPipelineError::AsModuleDescriptorError(err, processed));
                    }
                };
                entry.insert(Arc::new(
                    render_device.create_shader_module(&module_descriptor),
                ))
            }
        };

        Ok(module.clone())
    }

    fn clear(&mut self, handle: &Handle<Shader>) -> Vec<CachedPipelineId> {
        let mut shaders_to_clear = vec![handle.clone_weak()];
        let mut pipelines_to_queue = Vec::new();
        while let Some(handle) = shaders_to_clear.pop() {
            if let Some(data) = self.data.get_mut(&handle) {
                data.processed_shaders.clear();
                pipelines_to_queue.extend(data.pipelines.iter().cloned());
                shaders_to_clear.extend(data.dependents.iter().map(|h| h.clone_weak()));
            }
        }

        pipelines_to_queue
    }

    fn set_shader(&mut self, handle: &Handle<Shader>, shader: Shader) -> Vec<CachedPipelineId> {
        let pipelines_to_queue = self.clear(handle);
        if let Some(path) = shader.import_path() {
            self.import_path_shaders
                .insert(path.clone(), handle.clone_weak());
            if let Some(waiting_shaders) = self.waiting_on_import.get_mut(path) {
                for waiting_shader in waiting_shaders.drain(..) {
                    // resolve waiting shader import
                    let data = self.data.entry(waiting_shader.clone_weak()).or_default();
                    data.resolved_imports
                        .insert(path.clone(), handle.clone_weak());
                    // add waiting shader as dependent of this shader
                    let data = self.data.entry(handle.clone_weak()).or_default();
                    data.dependents.insert(waiting_shader.clone_weak());
                }
            }
        }

        for import in shader.imports() {
            if let Some(import_handle) = self.import_path_shaders.get(import) {
                // resolve import because it is currently available
                let data = self.data.entry(handle.clone_weak()).or_default();
                data.resolved_imports
                    .insert(import.clone(), import_handle.clone_weak());
                // add this shader as a dependent of the import
                let data = self.data.entry(import_handle.clone_weak()).or_default();
                data.dependents.insert(handle.clone_weak());
            } else {
                let waiting = self.waiting_on_import.entry(import.clone()).or_default();
                waiting.push(handle.clone_weak());
            }
        }

        self.shaders.insert(handle.clone_weak(), shader);
        pipelines_to_queue
    }

    fn remove(&mut self, handle: &Handle<Shader>) -> Vec<CachedPipelineId> {
        let pipelines_to_queue = self.clear(handle);
        if let Some(shader) = self.shaders.remove(handle) {
            if let Some(import_path) = shader.import_path() {
                self.import_path_shaders.remove(import_path);
            }
        }

        pipelines_to_queue
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
    waiting_pipelines: HashSet<CachedPipelineId>,
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
    #[error("{0}")]
    AsModuleDescriptorError(AsModuleDescriptorError, ProcessedShader),
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
    pub fn get_descriptor(&self, id: CachedPipelineId) -> &RenderPipelineDescriptor {
        &self.pipelines[id.0].descriptor
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
        self.waiting_pipelines.insert(id);
        id
    }

    fn set_shader(&mut self, handle: &Handle<Shader>, shader: &Shader) {
        let pipelines_to_queue = self.shader_cache.set_shader(handle, shader.clone());
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline.0].state = CachedPipelineState::Queued;
            self.waiting_pipelines.insert(cached_pipeline);
        }
    }

    fn remove_shader(&mut self, shader: &Handle<Shader>) {
        let pipelines_to_queue = self.shader_cache.remove(shader);
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline.0].state = CachedPipelineState::Queued;
            self.waiting_pipelines.insert(cached_pipeline);
        }
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
                        // shader could not be processed ... retrying won't help
                        RenderPipelineError::ProcessShaderError(err) => {
                            error!("failed to process shader: {}", err);
                            continue;
                        }
                        RenderPipelineError::AsModuleDescriptorError(err, source) => {
                            log_shader_error(source, err);
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
                    self.waiting_pipelines.insert(id);
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
                        self.waiting_pipelines.insert(id);
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
                .map(|layout| RawVertexBufferLayout {
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
                multiview: None,
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
        let mut cache = world.resource_mut::<Self>();
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

fn log_shader_error(source: &ProcessedShader, error: &AsModuleDescriptorError) {
    use codespan_reporting::{
        diagnostic::{Diagnostic, Label},
        files::SimpleFile,
        term,
    };

    match error {
        AsModuleDescriptorError::ShaderReflectError(error) => match error {
            ShaderReflectError::WgslParse(error) => {
                let source = source
                    .get_wgsl_source()
                    .expect("non-wgsl source for wgsl error");
                let msg = error.emit_to_string(source);
                error!("failed to process shader:\n{}", msg);
            }
            ShaderReflectError::GlslParse(errors) => {
                let source = source
                    .get_glsl_source()
                    .expect("non-glsl source for glsl error");
                let files = SimpleFile::new("glsl", source);
                let config = codespan_reporting::term::Config::default();
                let mut writer = term::termcolor::Ansi::new(Vec::new());

                for err in errors {
                    let mut diagnostic = Diagnostic::error().with_message(err.kind.to_string());

                    if let Some(range) = err.meta.to_range() {
                        diagnostic = diagnostic.with_labels(vec![Label::primary((), range)]);
                    }

                    term::emit(&mut writer, &config, &files, &diagnostic)
                        .expect("cannot write error");
                }

                let msg = writer.into_inner();
                let msg = String::from_utf8_lossy(&msg);

                error!("failed to process shader: \n{}", msg);
            }
            ShaderReflectError::SpirVParse(error) => {
                error!("failed to process shader:\n{}", error);
            }
            ShaderReflectError::Validation(error) => {
                let (filename, source) = match source {
                    ProcessedShader::Wgsl(source) => ("wgsl", source.as_ref()),
                    ProcessedShader::Glsl(source, _) => ("glsl", source.as_ref()),
                    ProcessedShader::SpirV(_) => {
                        error!("failed to process shader:\n{}", error);
                        return;
                    }
                };

                let files = SimpleFile::new(filename, source);
                let config = term::Config::default();
                let mut writer = term::termcolor::Ansi::new(Vec::new());

                let diagnostic = Diagnostic::error()
                    .with_message(error.to_string())
                    .with_labels(
                        error
                            .spans()
                            .map(|(span, desc)| {
                                Label::primary((), span.to_range().unwrap())
                                    .with_message(desc.to_owned())
                            })
                            .collect(),
                    )
                    .with_notes(
                        ErrorSources::of(error)
                            .map(|source| source.to_string())
                            .collect(),
                    );

                term::emit(&mut writer, &config, &files, &diagnostic).expect("cannot write error");

                let msg = writer.into_inner();
                let msg = String::from_utf8_lossy(&msg);

                error!("failed to process shader: \n{}", msg);
            }
        },
        AsModuleDescriptorError::WgslConversion(error) => {
            error!("failed to convert shader to wgsl: \n{}", error);
        }
        AsModuleDescriptorError::SpirVConversion(error) => {
            error!("failed to convert shader to spirv: \n{}", error);
        }
    }
}

struct ErrorSources<'a> {
    current: Option<&'a (dyn std::error::Error + 'static)>,
}

impl<'a> ErrorSources<'a> {
    fn of(error: &'a dyn std::error::Error) -> Self {
        Self {
            current: error.source(),
        }
    }
}

impl<'a> Iterator for ErrorSources<'a> {
    type Item = &'a (dyn std::error::Error + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        self.current = self.current.and_then(std::error::Error::source);
        current
    }
}
