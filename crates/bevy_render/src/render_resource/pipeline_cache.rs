use crate::{
    render_resource::{
        AsModuleDescriptorError, BindGroupLayout, BindGroupLayoutId, ComputePipeline,
        ComputePipelineDescriptor, ProcessShaderError, ProcessedShader,
        RawComputePipelineDescriptor, RawFragmentState, RawRenderPipelineDescriptor,
        RawVertexState, RenderPipeline, RenderPipelineDescriptor, Shader, ShaderImport,
        ShaderProcessor, ShaderReflectError,
    },
    renderer::RenderDevice,
    Extract,
};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::system::{Res, ResMut};
use bevy_ecs::{event::EventReader, system::Resource};
use bevy_utils::{
    default,
    tracing::{debug, error},
    Entry, HashMap, HashSet,
};
use std::{hash::Hash, iter::FusedIterator, mem, ops::Deref};
use thiserror::Error;
use wgpu::{PipelineLayoutDescriptor, VertexBufferLayout as RawVertexBufferLayout};

use crate::render_resource::resource_macros::*;

render_resource_wrapper!(ErasedShaderModule, wgpu::ShaderModule);
render_resource_wrapper!(ErasedPipelineLayout, wgpu::PipelineLayout);

/// A descriptor for a [`Pipeline`].
///
/// Used to store an heterogenous collection of render and compute pipeline descriptors together.
#[derive(Debug)]
pub enum PipelineDescriptor {
    RenderPipelineDescriptor(Box<RenderPipelineDescriptor>),
    ComputePipelineDescriptor(Box<ComputePipelineDescriptor>),
}

/// A pipeline defining the data layout and shader logic for a specific GPU task.
///
/// Used to store an heterogenous collection of render and compute pipelines together.
#[derive(Debug)]
pub enum Pipeline {
    RenderPipeline(RenderPipeline),
    ComputePipeline(ComputePipeline),
}

type CachedPipelineId = usize;

/// Index of a cached render pipeline in a [`PipelineCache`].
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct CachedRenderPipelineId(CachedPipelineId);

impl CachedRenderPipelineId {
    /// An invalid cached render pipeline index, often used to initialize a variable.
    pub const INVALID: Self = CachedRenderPipelineId(usize::MAX);
}

/// Index of a cached compute pipeline in a [`PipelineCache`].
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct CachedComputePipelineId(CachedPipelineId);

impl CachedComputePipelineId {
    /// An invalid cached compute pipeline index, often used to initialize a variable.
    pub const INVALID: Self = CachedComputePipelineId(usize::MAX);
}

pub struct CachedPipeline {
    pub descriptor: PipelineDescriptor,
    pub state: CachedPipelineState,
}

/// State of a cached pipeline inserted into a [`PipelineCache`].
#[derive(Debug)]
pub enum CachedPipelineState {
    /// The pipeline GPU object is queued for creation.
    Queued,
    /// The pipeline GPU object was created successfully and is available (allocated on the GPU).
    Ok(Pipeline),
    /// An error occurred while trying to create the pipeline GPU object.
    Err(PipelineCacheError),
}

impl CachedPipelineState {
    /// Convenience method to "unwrap" a pipeline state into its underlying GPU object.
    ///
    /// # Returns
    ///
    /// The method returns the allocated pipeline GPU object.
    ///
    /// # Panics
    ///
    /// This method panics if the pipeline GPU object is not available, either because it is
    /// pending creation or because an error occurred while attempting to create GPU object.
    pub fn unwrap(&self) -> &Pipeline {
        match self {
            CachedPipelineState::Ok(pipeline) => pipeline,
            CachedPipelineState::Queued => {
                panic!("Pipeline has not been compiled yet. It is still in the 'Queued' state.")
            }
            CachedPipelineState::Err(err) => panic!("{}", err),
        }
    }
}

#[derive(Default)]
struct ShaderData {
    pipelines: HashSet<CachedPipelineId>,
    processed_shaders: HashMap<Vec<ShaderDefVal>, ErasedShaderModule>,
    resolved_imports: HashMap<ShaderImport, Handle<Shader>>,
    dependents: HashSet<Handle<Shader>>,
}

#[derive(Default)]
struct ShaderCache {
    data: HashMap<Handle<Shader>, ShaderData>,
    shaders: HashMap<Handle<Shader>, Shader>,
    import_path_shaders: HashMap<ShaderImport, Handle<Shader>>,
    waiting_on_import: HashMap<ShaderImport, Vec<Handle<Shader>>>,
    processor: ShaderProcessor,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum ShaderDefVal {
    Bool(String, bool),
    Int(String, i32),
}

impl From<&str> for ShaderDefVal {
    fn from(key: &str) -> Self {
        ShaderDefVal::Bool(key.to_string(), true)
    }
}

impl From<String> for ShaderDefVal {
    fn from(key: String) -> Self {
        ShaderDefVal::Bool(key, true)
    }
}

impl ShaderCache {
    fn get(
        &mut self,
        render_device: &RenderDevice,
        pipeline: CachedPipelineId,
        handle: &Handle<Shader>,
        shader_defs: &[ShaderDefVal],
    ) -> Result<ErasedShaderModule, PipelineCacheError> {
        let shader = self
            .shaders
            .get(handle)
            .ok_or_else(|| PipelineCacheError::ShaderNotLoaded(handle.clone_weak()))?;
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
            return Err(PipelineCacheError::ShaderImportNotYetAvailable);
        }

        data.pipelines.insert(pipeline);

        // PERF: this shader_defs clone isn't great. use raw_entry_mut when it stabilizes
        let module = match data.processed_shaders.entry(shader_defs.to_vec()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let mut shader_defs = shader_defs.to_vec();
                #[cfg(feature = "webgl")]
                {
                    shader_defs.push("NO_ARRAY_TEXTURES_SUPPORT".into());
                    shader_defs.push("SIXTEEN_BYTE_ALIGNMENT".into());
                }

                shader_defs.push(ShaderDefVal::Int(
                    String::from("AVAILABLE_STORAGE_BUFFER_BINDINGS"),
                    render_device.limits().max_storage_buffers_per_shader_stage as i32,
                ));

                debug!(
                    "processing shader {:?}, with shader defs {:?}",
                    handle, shader_defs
                );
                let processed = self.processor.process(
                    shader,
                    &shader_defs,
                    &self.shaders,
                    &self.import_path_shaders,
                )?;
                let module_descriptor = match processed
                    .get_module_descriptor(render_device.features())
                {
                    Ok(module_descriptor) => module_descriptor,
                    Err(err) => {
                        return Err(PipelineCacheError::AsModuleDescriptorError(err, processed));
                    }
                };

                render_device
                    .wgpu_device()
                    .push_error_scope(wgpu::ErrorFilter::Validation);
                let shader_module = render_device.create_shader_module(module_descriptor);
                let error = render_device.wgpu_device().pop_error_scope();

                // `now_or_never` will return Some if the future is ready and None otherwise.
                // On native platforms, wgpu will yield the error immediatly while on wasm it may take longer since the browser APIs are asynchronous.
                // So to keep the complexity of the ShaderCache low, we will only catch this error early on native platforms,
                // and on wasm the error will be handled by wgpu and crash the application.
                if let Some(Some(wgpu::Error::Validation { description, .. })) =
                    bevy_utils::futures::now_or_never(error)
                {
                    return Err(PipelineCacheError::CreateShaderModule(description));
                }

                entry.insert(ErasedShaderModule::new(shader_module))
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
    layouts: HashMap<Vec<BindGroupLayoutId>, ErasedPipelineLayout>,
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
            ErasedPipelineLayout::new(render_device.create_pipeline_layout(
                &PipelineLayoutDescriptor {
                    bind_group_layouts: &bind_group_layouts,
                    ..default()
                },
            ))
        })
    }
}

/// Cache for render and compute pipelines.
///
/// The cache stores existing render and compute pipelines allocated on the GPU, as well as
/// pending creation. Pipelines inserted into the cache are identified by a unique ID, which
/// can be used to retrieve the actual GPU object once it's ready. The creation of the GPU
/// pipeline object is deferred to the [`RenderStage::Render`] stage, just before the render
/// graph starts being processed, as this requires access to the GPU.
///
/// Note that the cache do not perform automatic deduplication of identical pipelines. It is
/// up to the user not to insert the same pipeline twice to avoid wasting GPU resources.
///
/// [`RenderStage::Render`]: crate::RenderStage::Render
#[derive(Resource)]
pub struct PipelineCache {
    layout_cache: LayoutCache,
    shader_cache: ShaderCache,
    device: RenderDevice,
    pipelines: Vec<CachedPipeline>,
    waiting_pipelines: HashSet<CachedPipelineId>,
}

impl PipelineCache {
    pub fn pipelines(&self) -> impl Iterator<Item = &CachedPipeline> {
        self.pipelines.iter()
    }

    /// Create a new pipeline cache associated with the given render device.
    pub fn new(device: RenderDevice) -> Self {
        Self {
            device,
            layout_cache: default(),
            shader_cache: default(),
            waiting_pipelines: default(),
            pipelines: default(),
        }
    }

    /// Get the state of a cached render pipeline.
    ///
    /// See [`PipelineCache::queue_render_pipeline()`].
    #[inline]
    pub fn get_render_pipeline_state(&self, id: CachedRenderPipelineId) -> &CachedPipelineState {
        &self.pipelines[id.0].state
    }

    /// Get the state of a cached compute pipeline.
    ///
    /// See [`PipelineCache::queue_compute_pipeline()`].
    #[inline]
    pub fn get_compute_pipeline_state(&self, id: CachedComputePipelineId) -> &CachedPipelineState {
        &self.pipelines[id.0].state
    }

    /// Get the render pipeline descriptor a cached render pipeline was inserted from.
    ///
    /// See [`PipelineCache::queue_render_pipeline()`].
    #[inline]
    pub fn get_render_pipeline_descriptor(
        &self,
        id: CachedRenderPipelineId,
    ) -> &RenderPipelineDescriptor {
        match &self.pipelines[id.0].descriptor {
            PipelineDescriptor::RenderPipelineDescriptor(descriptor) => descriptor,
            PipelineDescriptor::ComputePipelineDescriptor(_) => unreachable!(),
        }
    }

    /// Get the compute pipeline descriptor a cached render pipeline was inserted from.
    ///
    /// See [`PipelineCache::queue_compute_pipeline()`].
    #[inline]
    pub fn get_compute_pipeline_descriptor(
        &self,
        id: CachedComputePipelineId,
    ) -> &ComputePipelineDescriptor {
        match &self.pipelines[id.0].descriptor {
            PipelineDescriptor::RenderPipelineDescriptor(_) => unreachable!(),
            PipelineDescriptor::ComputePipelineDescriptor(descriptor) => descriptor,
        }
    }

    /// Try to retrieve a render pipeline GPU object from a cached ID.
    ///
    /// # Returns
    ///
    /// This method returns a successfully created render pipeline if any, or `None` if the pipeline
    /// was not created yet or if there was an error during creation. You can check the actual creation
    /// state with [`PipelineCache::get_render_pipeline_state()`].
    #[inline]
    pub fn get_render_pipeline(&self, id: CachedRenderPipelineId) -> Option<&RenderPipeline> {
        if let CachedPipelineState::Ok(Pipeline::RenderPipeline(pipeline)) =
            &self.pipelines[id.0].state
        {
            Some(pipeline)
        } else {
            None
        }
    }

    /// Try to retrieve a compute pipeline GPU object from a cached ID.
    ///
    /// # Returns
    ///
    /// This method returns a successfully created compute pipeline if any, or `None` if the pipeline
    /// was not created yet or if there was an error during creation. You can check the actual creation
    /// state with [`PipelineCache::get_compute_pipeline_state()`].
    #[inline]
    pub fn get_compute_pipeline(&self, id: CachedComputePipelineId) -> Option<&ComputePipeline> {
        if let CachedPipelineState::Ok(Pipeline::ComputePipeline(pipeline)) =
            &self.pipelines[id.0].state
        {
            Some(pipeline)
        } else {
            None
        }
    }

    /// Insert a render pipeline into the cache, and queue its creation.
    ///
    /// The pipeline is always inserted and queued for creation. There is no attempt to deduplicate it with
    /// an already cached pipeline.
    ///
    /// # Returns
    ///
    /// This method returns the unique render shader ID of the cached pipeline, which can be used to query
    /// the caching state with [`get_render_pipeline_state()`] and to retrieve the created GPU pipeline once
    /// it's ready with [`get_render_pipeline()`].
    ///
    /// [`get_render_pipeline_state()`]: PipelineCache::get_render_pipeline_state
    /// [`get_render_pipeline()`]: PipelineCache::get_render_pipeline
    pub fn queue_render_pipeline(
        &mut self,
        descriptor: RenderPipelineDescriptor,
    ) -> CachedRenderPipelineId {
        let id = CachedRenderPipelineId(self.pipelines.len());
        self.pipelines.push(CachedPipeline {
            descriptor: PipelineDescriptor::RenderPipelineDescriptor(Box::new(descriptor)),
            state: CachedPipelineState::Queued,
        });
        self.waiting_pipelines.insert(id.0);
        id
    }

    /// Insert a compute pipeline into the cache, and queue its creation.
    ///
    /// The pipeline is always inserted and queued for creation. There is no attempt to deduplicate it with
    /// an already cached pipeline.
    ///
    /// # Returns
    ///
    /// This method returns the unique compute shader ID of the cached pipeline, which can be used to query
    /// the caching state with [`get_compute_pipeline_state()`] and to retrieve the created GPU pipeline once
    /// it's ready with [`get_compute_pipeline()`].
    ///
    /// [`get_compute_pipeline_state()`]: PipelineCache::get_compute_pipeline_state
    /// [`get_compute_pipeline()`]: PipelineCache::get_compute_pipeline
    pub fn queue_compute_pipeline(
        &mut self,
        descriptor: ComputePipelineDescriptor,
    ) -> CachedComputePipelineId {
        let id = CachedComputePipelineId(self.pipelines.len());
        self.pipelines.push(CachedPipeline {
            descriptor: PipelineDescriptor::ComputePipelineDescriptor(Box::new(descriptor)),
            state: CachedPipelineState::Queued,
        });
        self.waiting_pipelines.insert(id.0);
        id
    }

    fn set_shader(&mut self, handle: &Handle<Shader>, shader: &Shader) {
        let pipelines_to_queue = self.shader_cache.set_shader(handle, shader.clone());
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline].state = CachedPipelineState::Queued;
            self.waiting_pipelines.insert(cached_pipeline);
        }
    }

    fn remove_shader(&mut self, shader: &Handle<Shader>) {
        let pipelines_to_queue = self.shader_cache.remove(shader);
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline].state = CachedPipelineState::Queued;
            self.waiting_pipelines.insert(cached_pipeline);
        }
    }

    fn process_render_pipeline(
        &mut self,
        id: CachedPipelineId,
        descriptor: &RenderPipelineDescriptor,
    ) -> CachedPipelineState {
        let vertex_module = match self.shader_cache.get(
            &self.device,
            id,
            &descriptor.vertex.shader,
            &descriptor.vertex.shader_defs,
        ) {
            Ok(module) => module,
            Err(err) => {
                return CachedPipelineState::Err(err);
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
                    return CachedPipelineState::Err(err);
                }
            };
            Some((
                fragment_module,
                fragment.entry_point.deref(),
                fragment.targets.as_slice(),
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

        CachedPipelineState::Ok(Pipeline::RenderPipeline(pipeline))
    }

    fn process_compute_pipeline(
        &mut self,
        id: CachedPipelineId,
        descriptor: &ComputePipelineDescriptor,
    ) -> CachedPipelineState {
        let compute_module = match self.shader_cache.get(
            &self.device,
            id,
            &descriptor.shader,
            &descriptor.shader_defs,
        ) {
            Ok(module) => module,
            Err(err) => {
                return CachedPipelineState::Err(err);
            }
        };

        let layout = if let Some(layout) = &descriptor.layout {
            Some(self.layout_cache.get(&self.device, layout))
        } else {
            None
        };

        let descriptor = RawComputePipelineDescriptor {
            label: descriptor.label.as_deref(),
            layout,
            module: &compute_module,
            entry_point: descriptor.entry_point.as_ref(),
        };

        let pipeline = self.device.create_compute_pipeline(&descriptor);

        CachedPipelineState::Ok(Pipeline::ComputePipeline(pipeline))
    }

    /// Process the pipeline queue and create all pending pipelines if possible.
    ///
    /// This is generally called automatically during the [`RenderStage::Render`] stage, but can
    /// be called manually to force creation at a different time.
    ///
    /// [`RenderStage::Render`]: crate::RenderStage::Render
    pub fn process_queue(&mut self) {
        let waiting_pipelines = mem::take(&mut self.waiting_pipelines);
        let mut pipelines = mem::take(&mut self.pipelines);

        for id in waiting_pipelines {
            let pipeline = &mut pipelines[id];
            if matches!(pipeline.state, CachedPipelineState::Ok(_)) {
                continue;
            }

            pipeline.state = match &pipeline.descriptor {
                PipelineDescriptor::RenderPipelineDescriptor(descriptor) => {
                    self.process_render_pipeline(id, descriptor)
                }
                PipelineDescriptor::ComputePipelineDescriptor(descriptor) => {
                    self.process_compute_pipeline(id, descriptor)
                }
            };

            if let CachedPipelineState::Err(err) = &pipeline.state {
                match err {
                    PipelineCacheError::ShaderNotLoaded(_)
                    | PipelineCacheError::ShaderImportNotYetAvailable => {
                        // retry
                        self.waiting_pipelines.insert(id);
                    }
                    // shader could not be processed ... retrying won't help
                    PipelineCacheError::ProcessShaderError(err) => {
                        error!("failed to process shader: {}", err);
                        continue;
                    }
                    PipelineCacheError::AsModuleDescriptorError(err, source) => {
                        log_shader_error(source, err);
                        continue;
                    }
                    PipelineCacheError::CreateShaderModule(description) => {
                        error!("failed to create shader module: {}", description);
                        continue;
                    }
                }
            }
        }

        self.pipelines = pipelines;
    }

    pub(crate) fn process_pipeline_queue_system(mut cache: ResMut<Self>) {
        cache.process_queue();
    }

    pub(crate) fn extract_shaders(
        mut cache: ResMut<Self>,
        shaders: Extract<Res<Assets<Shader>>>,
        mut events: Extract<EventReader<AssetEvent<Shader>>>,
    ) {
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

/// Type of error returned by a [`PipelineCache`] when the creation of a GPU pipeline object failed.
#[derive(Error, Debug)]
pub enum PipelineCacheError {
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
    #[error("Could not create shader module: {0}")]
    CreateShaderModule(String),
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

impl<'a> FusedIterator for ErrorSources<'a> {}
