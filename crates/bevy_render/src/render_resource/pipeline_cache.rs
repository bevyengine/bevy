use crate::{render_resource::*, render_resource_wrapper, renderer::RenderDevice, Extract};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::{
    event::EventReader,
    system::{Res, ResMut, Resource},
};
use bevy_utils::{
    default,
    tracing::{debug, error},
    Entry, HashMap, HashSet,
};
use parking_lot::Mutex;
use std::{hash::Hash, iter::FusedIterator, mem};
use thiserror::Error;

render_resource_wrapper!(ErasedShaderModule, wgpu::ShaderModule);
render_resource_wrapper!(ErasedPipelineLayout, wgpu::PipelineLayout);

#[derive(Default)]
struct ShaderData {
    pipelines: HashSet<CachedPipelineId>,
    processed_shaders: HashMap<Vec<ShaderDefVal>, ErasedShaderModule>,
    resolved_imports: HashMap<ShaderImport, Handle<Shader>>,
    dependents: HashSet<Handle<Shader>>,
}

#[derive(Default)]
pub(crate) struct ShaderCache {
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
    UInt(String, u32),
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

impl ShaderDefVal {
    pub fn value_as_string(&self) -> String {
        match self {
            ShaderDefVal::Bool(_, def) => def.to_string(),
            ShaderDefVal::Int(_, def) => def.to_string(),
            ShaderDefVal::UInt(_, def) => def.to_string(),
        }
    }
}

impl ShaderCache {
    pub(crate) fn get(
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

                shader_defs.push(ShaderDefVal::UInt(
                    String::from("AVAILABLE_STORAGE_BUFFER_BINDINGS"),
                    render_device.limits().max_storage_buffers_per_shader_stage,
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
                // On native platforms, wgpu will yield the error immediately while on wasm it may take longer since the browser APIs are asynchronous.
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

type LayoutCacheKey = (Vec<BindGroupLayoutId>, Vec<PushConstantRange>);
#[derive(Default)]
pub(crate) struct LayoutCache {
    layouts: HashMap<LayoutCacheKey, ErasedPipelineLayout>,
}

impl LayoutCache {
    pub(crate) fn get(
        &mut self,
        render_device: &RenderDevice,
        bind_group_layouts: &[BindGroupLayout],
        push_constant_ranges: Vec<PushConstantRange>,
    ) -> &wgpu::PipelineLayout {
        let bind_group_ids = bind_group_layouts.iter().map(|l| l.id()).collect();
        self.layouts
            .entry((bind_group_ids, push_constant_ranges))
            .or_insert_with_key(|(_, push_constant_ranges)| {
                let bind_group_layouts = bind_group_layouts
                    .iter()
                    .map(|l| l.value())
                    .collect::<Vec<_>>();
                ErasedPipelineLayout::new(render_device.create_pipeline_layout(
                    &PipelineLayoutDescriptor {
                        bind_group_layouts: &bind_group_layouts,
                        push_constant_ranges,
                        ..default()
                    },
                ))
            })
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub(crate) enum CachedPipelineId {
    Render(RenderPipelineId),
    Compute(ComputePipelineId),
}

impl From<RenderPipelineId> for CachedPipelineId {
    fn from(value: RenderPipelineId) -> Self {
        Self::Render(value)
    }
}

impl From<ComputePipelineId> for CachedPipelineId {
    fn from(value: ComputePipelineId) -> Self {
        Self::Compute(value)
    }
}

struct CachedPipeline<I, D, P> {
    id: I,
    descriptor: D,
    state: PipelineState<P>,
}

struct PipelineCacheInternal<I: PipelineId, D, P> {
    pipelines: Vec<CachedPipeline<I, D, P>>,
    waiting_pipelines: HashSet<I>,
    new_pipelines: Mutex<Vec<CachedPipeline<I, D, P>>>,
}

impl<I: PipelineId, D, P> Default for PipelineCacheInternal<I, D, P> {
    fn default() -> Self {
        Self {
            pipelines: default(),
            waiting_pipelines: default(),
            new_pipelines: default(),
        }
    }
}

impl<I: PipelineId, D, P: Pipeline<I, D, P>> PipelineCacheInternal<I, D, P> {
    #[inline]
    fn pipeline_state(&self, id: I) -> &PipelineState<P> {
        &self.pipelines[id.index()].state
    }

    #[inline]
    fn pipeline_descriptor(&self, id: I) -> &D {
        &self.pipelines[id.index()].descriptor
    }

    #[inline]
    fn get_pipeline(&self, id: I) -> Option<&P> {
        match &self.pipelines[id.index()].state {
            PipelineState::Ok(pipeline) => Some(pipeline),
            _ => None,
        }
    }

    #[inline]
    fn reset_pipeline(&mut self, id: I) {
        self.pipelines[id.index()].state = PipelineState::Queued;
        self.waiting_pipelines.insert(id);
    }

    #[inline]
    fn queue_pipeline(&self, descriptor: D) -> I {
        let mut new_pipelines = self.new_pipelines.lock();
        let id = I::new((self.pipelines.len() + new_pipelines.len()) as u32);
        new_pipelines.push(CachedPipeline {
            id,
            descriptor,
            state: PipelineState::Queued,
        });
        id
    }

    fn process_queue(
        &mut self,
        device: &RenderDevice,
        shader_cache: &mut ShaderCache,
        layout_cache: &mut LayoutCache,
    ) {
        let mut waiting_pipelines = mem::take(&mut self.waiting_pipelines);
        let mut pipelines = mem::take(&mut self.pipelines);

        {
            let mut new_pipelines = self.new_pipelines.lock();
            for new_pipeline in new_pipelines.drain(..) {
                let id = new_pipeline.id;
                pipelines.push(new_pipeline);
                waiting_pipelines.insert(id);
            }
        }

        for id in waiting_pipelines {
            let pipeline = &mut pipelines[id.index()];
            if matches!(pipeline.state, PipelineState::Ok(_)) {
                continue;
            }

            pipeline.state =
                P::process_pipeline(id, &pipeline.descriptor, device, shader_cache, layout_cache);

            if let PipelineState::Err(err) = &pipeline.state {
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
}

/// State of a cached pipeline inserted into the [`PipelineCache`].
#[derive(Debug)]
pub enum PipelineState<P> {
    /// The pipeline GPU object is queued for creation.
    Queued,
    /// The pipeline GPU object was created successfully and is available (allocated on the GPU).
    Ok(P),
    /// An error occurred while trying to create the pipeline GPU object.
    Err(PipelineCacheError),
}

/// Cache for render and compute pipelines.
///
/// The cache stores existing render and compute pipelines allocated on the GPU, as well as
/// pending creation. Pipelines inserted into the cache are identified by a unique ID, which
/// can be used to retrieve the actual GPU object once it's ready. The creation of the GPU
/// pipeline object is deferred to the [`RenderSet::Render`] step, just before the render
/// graph starts being processed, as this requires access to the GPU.
///
/// Note that the cache do not perform automatic deduplication of identical pipelines. It is
/// up to the user not to insert the same pipeline twice to avoid wasting GPU resources.
///
/// [`RenderSet::Render`]: crate::RenderSet::Render
#[derive(Resource)]
pub struct PipelineCache {
    layout_cache: LayoutCache,
    shader_cache: ShaderCache,
    device: RenderDevice,
    render_pipeline_cache:
        PipelineCacheInternal<RenderPipelineId, RenderPipelineDescriptor, RenderPipeline>,
    compute_pipeline_cache:
        PipelineCacheInternal<ComputePipelineId, ComputePipelineDescriptor, ComputePipeline>,
}

impl PipelineCache {
    /// Create a new pipeline cache associated with the given render device.
    pub fn new(device: RenderDevice) -> Self {
        Self {
            device,
            layout_cache: default(),
            shader_cache: default(),
            render_pipeline_cache: default(),
            compute_pipeline_cache: default(),
        }
    }

    /// Get the state of the render pipeline.
    ///
    /// See [`PipelineCache::queue_render_pipeline()`].
    #[inline]
    pub fn render_pipeline_state(&self, id: RenderPipelineId) -> &PipelineState<RenderPipeline> {
        self.render_pipeline_cache.pipeline_state(id)
    }

    /// Get the state of the compute pipeline.
    ///
    /// See [`PipelineCache::queue_compute_pipeline()`].
    #[inline]
    pub fn compute_pipeline_state(&self, id: ComputePipelineId) -> &PipelineState<ComputePipeline> {
        self.compute_pipeline_cache.pipeline_state(id)
    }

    /// Get the render pipeline descriptor the render pipeline was inserted from.
    ///
    /// See [`PipelineCache::queue_render_pipeline()`].
    #[inline]
    pub fn render_pipeline_descriptor(&self, id: RenderPipelineId) -> &RenderPipelineDescriptor {
        self.render_pipeline_cache.pipeline_descriptor(id)
    }

    /// Get the compute pipeline descriptor the render pipeline was inserted from.
    ///
    /// See [`PipelineCache::queue_compute_pipeline()`].
    #[inline]
    pub fn compute_pipeline_descriptor(&self, id: ComputePipelineId) -> &ComputePipelineDescriptor {
        self.compute_pipeline_cache.pipeline_descriptor(id)
    }

    /// Try to retrieve a render pipeline GPU object.
    ///
    /// # Returns
    ///
    /// This method returns a successfully created render pipeline if any, or `None` if the pipeline
    /// was not created yet or if there was an error during creation. You can check the actual
    /// creation state with [`PipelineCache::render_pipeline_state()`].
    #[inline]
    pub fn get_render_pipeline(&self, id: RenderPipelineId) -> Option<&RenderPipeline> {
        self.render_pipeline_cache.get_pipeline(id)
    }

    /// Try to retrieve a compute pipeline GPU object.
    ///
    /// # Returns
    ///
    /// This method returns a successfully created compute pipeline if any, or `None` if the pipeline
    /// was not created yet or if there was an error during creation. You can check the actual
    /// creation state with [`PipelineCache::compute_pipeline_state()`].
    #[inline]
    pub fn get_compute_pipeline(&self, id: ComputePipelineId) -> Option<&ComputePipeline> {
        self.compute_pipeline_cache.get_pipeline(id)
    }

    /// Insert a render pipeline into the cache, and queue its creation.
    ///
    /// The pipeline is always inserted and queued for creation. There is no attempt to deduplicate
    /// it with an already cached pipeline.
    ///
    /// # Returns
    ///
    /// This method returns the unique render pipeline ID of the pipeline, which can be used to
    /// query the caching state with [`render_pipeline_state()`] and to retrieve the created GPU
    /// pipeline once it's ready with [`get_render_pipeline()`].
    ///
    /// [`render_pipeline_state()`]: PipelineCache::render_pipeline_state
    /// [`get_render_pipeline()`]: PipelineCache::get_render_pipeline
    pub fn queue_render_pipeline(&self, descriptor: RenderPipelineDescriptor) -> RenderPipelineId {
        self.render_pipeline_cache.queue_pipeline(descriptor)
    }

    /// Insert a compute pipeline into the cache, and queue its creation.
    ///
    /// The pipeline is always inserted and queued for creation. There is no attempt to deduplicate
    /// it with an already cached pipeline.
    ///
    /// # Returns
    ///
    /// This method returns the unique compute pipeline ID of the pipeline, which can be used to
    /// query the caching state with [`compute_pipeline_state()`] and to retrieve the created GPU
    /// pipeline once it's ready with [`get_compute_pipeline()`].
    ///
    /// [`compute_pipeline_state()`]: PipelineCache::compute_pipeline_state
    /// [`get_compute_pipeline()`]: PipelineCache::get_compute_pipeline
    pub fn queue_compute_pipeline(
        &self,
        descriptor: ComputePipelineDescriptor,
    ) -> ComputePipelineId {
        self.compute_pipeline_cache.queue_pipeline(descriptor)
    }

    fn set_shader(&mut self, handle: &Handle<Shader>, shader: &Shader) {
        let pipelines_to_queue = self.shader_cache.set_shader(handle, shader.clone());
        for id in pipelines_to_queue {
            match id {
                CachedPipelineId::Render(id) => self.render_pipeline_cache.reset_pipeline(id),
                CachedPipelineId::Compute(id) => self.compute_pipeline_cache.reset_pipeline(id),
            }
        }
    }

    fn remove_shader(&mut self, shader: &Handle<Shader>) {
        let pipelines_to_queue = self.shader_cache.remove(shader);
        for id in pipelines_to_queue {
            match id {
                CachedPipelineId::Render(id) => self.render_pipeline_cache.reset_pipeline(id),
                CachedPipelineId::Compute(id) => self.compute_pipeline_cache.reset_pipeline(id),
            }
        }
    }

    /// Process the pipeline queue and create all pending pipelines if possible.
    ///
    /// This is generally called automatically during the [`RenderSet::Render`] step, but can
    /// be called manually to force creation at a different time.
    ///
    /// [`RenderSet::Render`]: crate::RenderSet::Render
    pub fn process_queue(&mut self) {
        let Self {
            layout_cache,
            shader_cache,
            ref device,
            render_pipeline_cache,
            compute_pipeline_cache,
        } = self;

        render_pipeline_cache.process_queue(device, shader_cache, layout_cache);
        compute_pipeline_cache.process_queue(device, shader_cache, layout_cache);
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
        "Pipeline could not be compiled because the following shader is not loaded yet: {0:?}"
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
