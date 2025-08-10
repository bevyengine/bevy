use crate::WgpuWrapper;
use crate::{
    render_resource::*,
    renderer::{RenderAdapter, RenderDevice},
    Extract,
};
use alloc::{borrow::Cow, sync::Arc};
use bevy_asset::{AssetEvent, AssetId, Assets, Handle};
use bevy_ecs::{
    event::EventReader,
    resource::Resource,
    system::{Res, ResMut},
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_shader::{
    CachedPipelineId, PipelineCacheError, Shader, ShaderCache, ShaderCacheSource, ShaderDefVal,
    ValidateShader,
};
use bevy_tasks::Task;
use bevy_utils::default;
use core::{future::Future, hash::Hash, mem};
use std::sync::{Mutex, PoisonError};
use tracing::error;
use wgpu::{PipelineCompilationOptions, VertexBufferLayout as RawVertexBufferLayout};

/// A descriptor for a [`Pipeline`].
///
/// Used to store a heterogenous collection of render and compute pipeline descriptors together.
#[derive(Debug)]
pub enum PipelineDescriptor {
    RenderPipelineDescriptor(Box<RenderPipelineDescriptor>),
    ComputePipelineDescriptor(Box<ComputePipelineDescriptor>),
}

/// A pipeline defining the data layout and shader logic for a specific GPU task.
///
/// Used to store a heterogenous collection of render and compute pipelines together.
#[derive(Debug)]
pub enum Pipeline {
    RenderPipeline(RenderPipeline),
    ComputePipeline(ComputePipeline),
}

/// Index of a cached render pipeline in a [`PipelineCache`].
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct CachedRenderPipelineId(CachedPipelineId);

impl CachedRenderPipelineId {
    /// An invalid cached render pipeline index, often used to initialize a variable.
    pub const INVALID: Self = CachedRenderPipelineId(usize::MAX);

    #[inline]
    pub fn id(&self) -> usize {
        self.0
    }
}

/// Index of a cached compute pipeline in a [`PipelineCache`].
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct CachedComputePipelineId(CachedPipelineId);

impl CachedComputePipelineId {
    /// An invalid cached compute pipeline index, often used to initialize a variable.
    pub const INVALID: Self = CachedComputePipelineId(usize::MAX);

    #[inline]
    pub fn id(&self) -> usize {
        self.0
    }
}

pub struct CachedPipeline {
    pub descriptor: PipelineDescriptor,
    pub state: CachedPipelineState,
}

/// State of a cached pipeline inserted into a [`PipelineCache`].
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        clippy::large_enum_variant,
        reason = "See https://github.com/bevyengine/bevy/issues/19220"
    )
)]
#[derive(Debug)]
pub enum CachedPipelineState {
    /// The pipeline GPU object is queued for creation.
    Queued,
    /// The pipeline GPU object is being created.
    Creating(Task<Result<Pipeline, PipelineCacheError>>),
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
            CachedPipelineState::Creating(..) => {
                panic!("Pipeline has not been compiled yet. It is still in the 'Creating' state.")
            }
            CachedPipelineState::Err(err) => panic!("{}", err),
        }
    }
}

type LayoutCacheKey = (Vec<BindGroupLayoutId>, Vec<PushConstantRange>);
#[derive(Default)]
struct LayoutCache {
    layouts: HashMap<LayoutCacheKey, Arc<WgpuWrapper<PipelineLayout>>>,
}

impl LayoutCache {
    fn get(
        &mut self,
        render_device: &RenderDevice,
        bind_group_layouts: &[BindGroupLayout],
        push_constant_ranges: Vec<PushConstantRange>,
    ) -> Arc<WgpuWrapper<PipelineLayout>> {
        let bind_group_ids = bind_group_layouts.iter().map(BindGroupLayout::id).collect();
        self.layouts
            .entry((bind_group_ids, push_constant_ranges))
            .or_insert_with_key(|(_, push_constant_ranges)| {
                let bind_group_layouts = bind_group_layouts
                    .iter()
                    .map(BindGroupLayout::value)
                    .collect::<Vec<_>>();
                Arc::new(WgpuWrapper::new(render_device.create_pipeline_layout(
                    &PipelineLayoutDescriptor {
                        bind_group_layouts: &bind_group_layouts,
                        push_constant_ranges,
                        ..default()
                    },
                )))
            })
            .clone()
    }
}

#[expect(
    clippy::result_large_err,
    reason = "See https://github.com/bevyengine/bevy/issues/19220"
)]
fn load_module(
    render_device: &RenderDevice,
    shader_source: ShaderCacheSource,
    validate_shader: &ValidateShader,
) -> Result<WgpuWrapper<ShaderModule>, PipelineCacheError> {
    let shader_source = match shader_source {
        #[cfg(feature = "shader_format_spirv")]
        ShaderCacheSource::SpirV(data) => wgpu::util::make_spirv(data),
        #[cfg(not(feature = "shader_format_spirv"))]
        ShaderCacheSource::SpirV(_) => {
            unimplemented!("Enable feature \"shader_format_spirv\" to use SPIR-V shaders")
        }
        ShaderCacheSource::Wgsl(src) => ShaderSource::Wgsl(Cow::Owned(src)),
        #[cfg(not(feature = "decoupled_naga"))]
        ShaderCacheSource::Naga(src) => ShaderSource::Naga(Cow::Owned(src)),
    };
    let module_descriptor = ShaderModuleDescriptor {
        label: None,
        source: shader_source,
    };

    render_device
        .wgpu_device()
        .push_error_scope(wgpu::ErrorFilter::Validation);

    let shader_module = WgpuWrapper::new(match validate_shader {
        ValidateShader::Enabled => {
            render_device.create_and_validate_shader_module(module_descriptor)
        }
        // SAFETY: we are interfacing with shader code, which may contain undefined behavior,
        // such as indexing out of bounds.
        // The checks required are prohibitively expensive and a poor default for game engines.
        ValidateShader::Disabled => unsafe {
            render_device.create_shader_module(module_descriptor)
        },
    });

    let error = render_device.wgpu_device().pop_error_scope();

    // `now_or_never` will return Some if the future is ready and None otherwise.
    // On native platforms, wgpu will yield the error immediately while on wasm it may take longer since the browser APIs are asynchronous.
    // So to keep the complexity of the ShaderCache low, we will only catch this error early on native platforms,
    // and on wasm the error will be handled by wgpu and crash the application.
    if let Some(Some(wgpu::Error::Validation { description, .. })) =
        bevy_tasks::futures::now_or_never(error)
    {
        return Err(PipelineCacheError::CreateShaderModule(description));
    }

    Ok(shader_module)
}

/// Cache for render and compute pipelines.
///
/// The cache stores existing render and compute pipelines allocated on the GPU, as well as
/// pending creation. Pipelines inserted into the cache are identified by a unique ID, which
/// can be used to retrieve the actual GPU object once it's ready. The creation of the GPU
/// pipeline object is deferred to the [`RenderSystems::Render`] step, just before the render
/// graph starts being processed, as this requires access to the GPU.
///
/// Note that the cache does not perform automatic deduplication of identical pipelines. It is
/// up to the user not to insert the same pipeline twice to avoid wasting GPU resources.
///
/// [`RenderSystems::Render`]: crate::RenderSystems::Render
#[derive(Resource)]
pub struct PipelineCache {
    layout_cache: Arc<Mutex<LayoutCache>>,
    shader_cache: Arc<Mutex<ShaderCache<WgpuWrapper<ShaderModule>, RenderDevice>>>,
    device: RenderDevice,
    pipelines: Vec<CachedPipeline>,
    waiting_pipelines: HashSet<CachedPipelineId>,
    new_pipelines: Mutex<Vec<CachedPipeline>>,
    global_shader_defs: Vec<ShaderDefVal>,
    /// If `true`, disables asynchronous pipeline compilation.
    /// This has no effect on macOS, wasm, or without the `multi_threaded` feature.
    synchronous_pipeline_compilation: bool,
}

impl PipelineCache {
    /// Returns an iterator over the pipelines in the pipeline cache.
    pub fn pipelines(&self) -> impl Iterator<Item = &CachedPipeline> {
        self.pipelines.iter()
    }

    /// Returns a iterator of the IDs of all currently waiting pipelines.
    pub fn waiting_pipelines(&self) -> impl Iterator<Item = CachedPipelineId> + '_ {
        self.waiting_pipelines.iter().copied()
    }

    /// Create a new pipeline cache associated with the given render device.
    pub fn new(
        device: RenderDevice,
        render_adapter: RenderAdapter,
        synchronous_pipeline_compilation: bool,
    ) -> Self {
        let mut global_shader_defs = Vec::new();
        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        {
            global_shader_defs.push("NO_ARRAY_TEXTURES_SUPPORT".into());
            global_shader_defs.push("NO_CUBE_ARRAY_TEXTURES_SUPPORT".into());
            global_shader_defs.push("SIXTEEN_BYTE_ALIGNMENT".into());
        }

        if cfg!(target_abi = "sim") {
            global_shader_defs.push("NO_CUBE_ARRAY_TEXTURES_SUPPORT".into());
        }

        global_shader_defs.push(ShaderDefVal::UInt(
            String::from("AVAILABLE_STORAGE_BUFFER_BINDINGS"),
            device.limits().max_storage_buffers_per_shader_stage,
        ));

        Self {
            shader_cache: Arc::new(Mutex::new(ShaderCache::new(
                device.features(),
                render_adapter.get_downlevel_capabilities().flags,
                load_module,
            ))),
            device,
            layout_cache: default(),
            waiting_pipelines: default(),
            new_pipelines: default(),
            pipelines: default(),
            global_shader_defs,
            synchronous_pipeline_compilation,
        }
    }

    /// Get the state of a cached render pipeline.
    ///
    /// See [`PipelineCache::queue_render_pipeline()`].
    #[inline]
    pub fn get_render_pipeline_state(&self, id: CachedRenderPipelineId) -> &CachedPipelineState {
        // If the pipeline id isn't in `pipelines`, it's queued in `new_pipelines`
        self.pipelines
            .get(id.0)
            .map_or(&CachedPipelineState::Queued, |pipeline| &pipeline.state)
    }

    /// Get the state of a cached compute pipeline.
    ///
    /// See [`PipelineCache::queue_compute_pipeline()`].
    #[inline]
    pub fn get_compute_pipeline_state(&self, id: CachedComputePipelineId) -> &CachedPipelineState {
        // If the pipeline id isn't in `pipelines`, it's queued in `new_pipelines`
        self.pipelines
            .get(id.0)
            .map_or(&CachedPipelineState::Queued, |pipeline| &pipeline.state)
    }

    /// Get the render pipeline descriptor a cached render pipeline was inserted from.
    ///
    /// See [`PipelineCache::queue_render_pipeline()`].
    ///
    /// **Note**: Be careful calling this method. It will panic if called with a pipeline that
    /// has been queued but has not yet been processed by [`PipelineCache::process_queue()`].
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
    ///
    /// **Note**: Be careful calling this method. It will panic if called with a pipeline that
    /// has been queued but has not yet been processed by [`PipelineCache::process_queue()`].
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
            &self.pipelines.get(id.0)?.state
        {
            Some(pipeline)
        } else {
            None
        }
    }

    /// Wait for a render pipeline to finish compiling.
    #[inline]
    pub fn block_on_render_pipeline(&mut self, id: CachedRenderPipelineId) {
        if self.pipelines.len() <= id.0 {
            self.process_queue();
        }

        let state = &mut self.pipelines[id.0].state;
        if let CachedPipelineState::Creating(task) = state {
            *state = match bevy_tasks::block_on(task) {
                Ok(p) => CachedPipelineState::Ok(p),
                Err(e) => CachedPipelineState::Err(e),
            };
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
            &self.pipelines.get(id.0)?.state
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
        &self,
        descriptor: RenderPipelineDescriptor,
    ) -> CachedRenderPipelineId {
        let mut new_pipelines = self
            .new_pipelines
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let id = CachedRenderPipelineId(self.pipelines.len() + new_pipelines.len());
        new_pipelines.push(CachedPipeline {
            descriptor: PipelineDescriptor::RenderPipelineDescriptor(Box::new(descriptor)),
            state: CachedPipelineState::Queued,
        });
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
        &self,
        descriptor: ComputePipelineDescriptor,
    ) -> CachedComputePipelineId {
        let mut new_pipelines = self
            .new_pipelines
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let id = CachedComputePipelineId(self.pipelines.len() + new_pipelines.len());
        new_pipelines.push(CachedPipeline {
            descriptor: PipelineDescriptor::ComputePipelineDescriptor(Box::new(descriptor)),
            state: CachedPipelineState::Queued,
        });
        id
    }

    fn set_shader(&mut self, id: AssetId<Shader>, shader: Shader) {
        let mut shader_cache = self.shader_cache.lock().unwrap();
        let pipelines_to_queue = shader_cache.set_shader(id, shader);
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline].state = CachedPipelineState::Queued;
            self.waiting_pipelines.insert(cached_pipeline);
        }
    }

    fn remove_shader(&mut self, shader: AssetId<Shader>) {
        let mut shader_cache = self.shader_cache.lock().unwrap();
        let pipelines_to_queue = shader_cache.remove(shader);
        for cached_pipeline in pipelines_to_queue {
            self.pipelines[cached_pipeline].state = CachedPipelineState::Queued;
            self.waiting_pipelines.insert(cached_pipeline);
        }
    }

    fn start_create_render_pipeline(
        &mut self,
        id: CachedPipelineId,
        descriptor: RenderPipelineDescriptor,
    ) -> CachedPipelineState {
        let device = self.device.clone();
        let shader_cache = self.shader_cache.clone();
        let layout_cache = self.layout_cache.clone();

        create_pipeline_task(
            async move {
                let mut shader_cache = shader_cache.lock().unwrap();
                let mut layout_cache = layout_cache.lock().unwrap();

                let vertex_module = match shader_cache.get(
                    &device,
                    id,
                    descriptor.vertex.shader.id(),
                    &descriptor.vertex.shader_defs,
                ) {
                    Ok(module) => module,
                    Err(err) => return Err(err),
                };

                let fragment_module = match &descriptor.fragment {
                    Some(fragment) => {
                        match shader_cache.get(
                            &device,
                            id,
                            fragment.shader.id(),
                            &fragment.shader_defs,
                        ) {
                            Ok(module) => Some(module),
                            Err(err) => return Err(err),
                        }
                    }
                    None => None,
                };

                let layout =
                    if descriptor.layout.is_empty() && descriptor.push_constant_ranges.is_empty() {
                        None
                    } else {
                        Some(layout_cache.get(
                            &device,
                            &descriptor.layout,
                            descriptor.push_constant_ranges.to_vec(),
                        ))
                    };

                drop((shader_cache, layout_cache));

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

                let fragment_data = descriptor.fragment.as_ref().map(|fragment| {
                    (
                        fragment_module.unwrap(),
                        fragment.entry_point.as_deref(),
                        fragment.targets.as_slice(),
                    )
                });

                // TODO: Expose the rest of this somehow
                let compilation_options = PipelineCompilationOptions {
                    constants: &[],
                    zero_initialize_workgroup_memory: descriptor.zero_initialize_workgroup_memory,
                };

                let descriptor = RawRenderPipelineDescriptor {
                    multiview: None,
                    depth_stencil: descriptor.depth_stencil.clone(),
                    label: descriptor.label.as_deref(),
                    layout: layout.as_ref().map(|layout| -> &PipelineLayout { layout }),
                    multisample: descriptor.multisample,
                    primitive: descriptor.primitive,
                    vertex: RawVertexState {
                        buffers: &vertex_buffer_layouts,
                        entry_point: descriptor.vertex.entry_point.as_deref(),
                        module: &vertex_module,
                        // TODO: Should this be the same as the fragment compilation options?
                        compilation_options: compilation_options.clone(),
                    },
                    fragment: fragment_data
                        .as_ref()
                        .map(|(module, entry_point, targets)| RawFragmentState {
                            entry_point: entry_point.as_deref(),
                            module,
                            targets,
                            // TODO: Should this be the same as the vertex compilation options?
                            compilation_options,
                        }),
                    cache: None,
                };

                Ok(Pipeline::RenderPipeline(
                    device.create_render_pipeline(&descriptor),
                ))
            },
            self.synchronous_pipeline_compilation,
        )
    }

    fn start_create_compute_pipeline(
        &mut self,
        id: CachedPipelineId,
        descriptor: ComputePipelineDescriptor,
    ) -> CachedPipelineState {
        let device = self.device.clone();
        let shader_cache = self.shader_cache.clone();
        let layout_cache = self.layout_cache.clone();

        create_pipeline_task(
            async move {
                let mut shader_cache = shader_cache.lock().unwrap();
                let mut layout_cache = layout_cache.lock().unwrap();

                let compute_module = match shader_cache.get(
                    &device,
                    id,
                    descriptor.shader.id(),
                    &descriptor.shader_defs,
                ) {
                    Ok(module) => module,
                    Err(err) => return Err(err),
                };

                let layout =
                    if descriptor.layout.is_empty() && descriptor.push_constant_ranges.is_empty() {
                        None
                    } else {
                        Some(layout_cache.get(
                            &device,
                            &descriptor.layout,
                            descriptor.push_constant_ranges.to_vec(),
                        ))
                    };

                drop((shader_cache, layout_cache));

                let descriptor = RawComputePipelineDescriptor {
                    label: descriptor.label.as_deref(),
                    layout: layout.as_ref().map(|layout| -> &PipelineLayout { layout }),
                    module: &compute_module,
                    entry_point: descriptor.entry_point.as_deref(),
                    // TODO: Expose the rest of this somehow
                    compilation_options: PipelineCompilationOptions {
                        constants: &[],
                        zero_initialize_workgroup_memory: descriptor
                            .zero_initialize_workgroup_memory,
                    },
                    cache: None,
                };

                Ok(Pipeline::ComputePipeline(
                    device.create_compute_pipeline(&descriptor),
                ))
            },
            self.synchronous_pipeline_compilation,
        )
    }

    /// Process the pipeline queue and create all pending pipelines if possible.
    ///
    /// This is generally called automatically during the [`RenderSystems::Render`] step, but can
    /// be called manually to force creation at a different time.
    ///
    /// [`RenderSystems::Render`]: crate::RenderSystems::Render
    pub fn process_queue(&mut self) {
        let mut waiting_pipelines = mem::take(&mut self.waiting_pipelines);
        let mut pipelines = mem::take(&mut self.pipelines);

        {
            let mut new_pipelines = self
                .new_pipelines
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            for new_pipeline in new_pipelines.drain(..) {
                let id = pipelines.len();
                pipelines.push(new_pipeline);
                waiting_pipelines.insert(id);
            }
        }

        for id in waiting_pipelines {
            self.process_pipeline(&mut pipelines[id], id);
        }

        self.pipelines = pipelines;
    }

    fn process_pipeline(&mut self, cached_pipeline: &mut CachedPipeline, id: usize) {
        match &mut cached_pipeline.state {
            CachedPipelineState::Queued => {
                cached_pipeline.state = match &cached_pipeline.descriptor {
                    PipelineDescriptor::RenderPipelineDescriptor(descriptor) => {
                        self.start_create_render_pipeline(id, *descriptor.clone())
                    }
                    PipelineDescriptor::ComputePipelineDescriptor(descriptor) => {
                        self.start_create_compute_pipeline(id, *descriptor.clone())
                    }
                };
            }

            CachedPipelineState::Creating(task) => match bevy_tasks::futures::check_ready(task) {
                Some(Ok(pipeline)) => {
                    cached_pipeline.state = CachedPipelineState::Ok(pipeline);
                    return;
                }
                Some(Err(err)) => cached_pipeline.state = CachedPipelineState::Err(err),
                _ => (),
            },

            CachedPipelineState::Err(err) => match err {
                // Retry
                PipelineCacheError::ShaderNotLoaded(_)
                | PipelineCacheError::ShaderImportNotYetAvailable => {
                    cached_pipeline.state = CachedPipelineState::Queued;
                }

                // Shader could not be processed ... retrying won't help
                PipelineCacheError::ProcessShaderError(err) => {
                    let error_detail =
                        err.emit_to_string(&self.shader_cache.lock().unwrap().composer);
                    if std::env::var("VERBOSE_SHADER_ERROR")
                        .is_ok_and(|v| !(v.is_empty() || v == "0" || v == "false"))
                    {
                        error!("{}", pipeline_error_context(cached_pipeline));
                    }
                    error!("failed to process shader error:\n{}", error_detail);
                    return;
                }
                PipelineCacheError::CreateShaderModule(description) => {
                    error!("failed to create shader module: {}", description);
                    return;
                }
            },

            CachedPipelineState::Ok(_) => return,
        }

        // Retry
        self.waiting_pipelines.insert(id);
    }

    pub(crate) fn process_pipeline_queue_system(mut cache: ResMut<Self>) {
        cache.process_queue();
    }

    pub(crate) fn extract_shaders(
        mut cache: ResMut<Self>,
        shaders: Extract<Res<Assets<Shader>>>,
        mut events: Extract<EventReader<AssetEvent<Shader>>>,
    ) {
        for event in events.read() {
            #[expect(
                clippy::match_same_arms,
                reason = "LoadedWithDependencies is marked as a TODO, so it's likely this will no longer lint soon."
            )]
            match event {
                // PERF: Instead of blocking waiting for the shader cache lock, try again next frame if the lock is currently held
                AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                    if let Some(shader) = shaders.get(*id) {
                        let mut shader = shader.clone();
                        shader.shader_defs.extend(cache.global_shader_defs.clone());

                        cache.set_shader(*id, shader);
                    }
                }
                AssetEvent::Removed { id } => cache.remove_shader(*id),
                AssetEvent::Unused { .. } => {}
                AssetEvent::LoadedWithDependencies { .. } => {
                    // TODO: handle this
                }
            }
        }
    }
}

fn pipeline_error_context(cached_pipeline: &CachedPipeline) -> String {
    fn format(
        shader: &Handle<Shader>,
        entry: &Option<Cow<'static, str>>,
        shader_defs: &[ShaderDefVal],
    ) -> String {
        let source = match shader.path() {
            Some(path) => path.path().to_string_lossy().to_string(),
            None => String::new(),
        };
        let entry = match entry {
            Some(entry) => entry.to_string(),
            None => String::new(),
        };
        let shader_defs = shader_defs
            .iter()
            .flat_map(|def| match def {
                ShaderDefVal::Bool(k, v) if *v => Some(k.to_string()),
                ShaderDefVal::Int(k, v) => Some(format!("{k} = {v}")),
                ShaderDefVal::UInt(k, v) => Some(format!("{k} = {v}")),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{source}:{entry}\nshader defs: {shader_defs}")
    }
    match &cached_pipeline.descriptor {
        PipelineDescriptor::RenderPipelineDescriptor(desc) => {
            let vert = &desc.vertex;
            let vert_str = format(&vert.shader, &vert.entry_point, &vert.shader_defs);
            let Some(frag) = desc.fragment.as_ref() else {
                return vert_str;
            };
            let frag_str = format(&frag.shader, &frag.entry_point, &frag.shader_defs);
            format!("vertex {vert_str}\nfragment {frag_str}")
        }
        PipelineDescriptor::ComputePipelineDescriptor(desc) => {
            format(&desc.shader, &desc.entry_point, &desc.shader_defs)
        }
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "macos"),
    feature = "multi_threaded"
))]
fn create_pipeline_task(
    task: impl Future<Output = Result<Pipeline, PipelineCacheError>> + Send + 'static,
    sync: bool,
) -> CachedPipelineState {
    if !sync {
        return CachedPipelineState::Creating(bevy_tasks::AsyncComputeTaskPool::get().spawn(task));
    }

    match futures_lite::future::block_on(task) {
        Ok(pipeline) => CachedPipelineState::Ok(pipeline),
        Err(err) => CachedPipelineState::Err(err),
    }
}

#[cfg(any(
    target_arch = "wasm32",
    target_os = "macos",
    not(feature = "multi_threaded")
))]
fn create_pipeline_task(
    task: impl Future<Output = Result<Pipeline, PipelineCacheError>> + Send + 'static,
    _sync: bool,
) -> CachedPipelineState {
    match futures_lite::future::block_on(task) {
        Ok(pipeline) => CachedPipelineState::Ok(pipeline),
        Err(err) => CachedPipelineState::Err(err),
    }
}
