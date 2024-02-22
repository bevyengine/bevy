use crate::{render_resource::*, renderer::RenderDevice, Extract};
use bevy_asset::{AssetEvent, AssetId, Assets};
use bevy_ecs::system::{Res, ResMut};
use bevy_ecs::{event::EventReader, system::Resource};
use bevy_tasks::Task;
use bevy_utils::{
    default,
    tracing::{debug, error},
    Entry, HashMap, HashSet,
};
use naga::valid::Capabilities;
use std::{
    borrow::Cow,
    future::Future,
    hash::Hash,
    mem,
    ops::Deref,
    sync::{Arc, Mutex, PoisonError},
};
use thiserror::Error;
#[cfg(feature = "shader_format_spirv")]
use wgpu::util::make_spirv;
use wgpu::{
    Features, PipelineLayoutDescriptor, PushConstantRange, ShaderModuleDescriptor,
    VertexBufferLayout as RawVertexBufferLayout,
};

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

#[derive(Default)]
struct ShaderData {
    pipelines: HashSet<CachedPipelineId>,
    processed_shaders: HashMap<Vec<ShaderDefVal>, ErasedShaderModule>,
    resolved_imports: HashMap<ShaderImport, AssetId<Shader>>,
    dependents: HashSet<AssetId<Shader>>,
}

struct ShaderCache {
    data: HashMap<AssetId<Shader>, ShaderData>,
    shaders: HashMap<AssetId<Shader>, Shader>,
    import_path_shaders: HashMap<ShaderImport, AssetId<Shader>>,
    waiting_on_import: HashMap<ShaderImport, Vec<AssetId<Shader>>>,
    composer: naga_oil::compose::Composer,
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
    fn new(render_device: &RenderDevice) -> Self {
        const CAPABILITIES: &[(Features, Capabilities)] = &[
            (Features::PUSH_CONSTANTS, Capabilities::PUSH_CONSTANT),
            (Features::SHADER_F64, Capabilities::FLOAT64),
            (
                Features::SHADER_PRIMITIVE_INDEX,
                Capabilities::PRIMITIVE_INDEX,
            ),
            (
                Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                Capabilities::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
            ),
            (
                Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                Capabilities::SAMPLER_NON_UNIFORM_INDEXING,
            ),
            (
                Features::UNIFORM_BUFFER_AND_STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING,
                Capabilities::UNIFORM_BUFFER_AND_STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING,
            ),
        ];
        let features = render_device.features();
        let mut capabilities = Capabilities::empty();
        for (feature, capability) in CAPABILITIES {
            if features.contains(*feature) {
                capabilities |= *capability;
            }
        }

        // TODO: Check if this is supported, though I'm not sure if bevy works without this feature?
        // We can't compile for native at least without it.
        capabilities |= Capabilities::CUBE_ARRAY_TEXTURES;

        #[cfg(debug_assertions)]
        let composer = naga_oil::compose::Composer::default();
        #[cfg(not(debug_assertions))]
        let composer = naga_oil::compose::Composer::non_validating();

        let composer = composer.with_capabilities(capabilities);

        Self {
            composer,
            data: Default::default(),
            shaders: Default::default(),
            import_path_shaders: Default::default(),
            waiting_on_import: Default::default(),
        }
    }

    fn add_import_to_composer(
        composer: &mut naga_oil::compose::Composer,
        import_path_shaders: &HashMap<ShaderImport, AssetId<Shader>>,
        shaders: &HashMap<AssetId<Shader>, Shader>,
        import: &ShaderImport,
    ) -> Result<(), PipelineCacheError> {
        if !composer.contains_module(&import.module_name()) {
            if let Some(shader_handle) = import_path_shaders.get(import) {
                if let Some(shader) = shaders.get(shader_handle) {
                    for import in &shader.imports {
                        Self::add_import_to_composer(
                            composer,
                            import_path_shaders,
                            shaders,
                            import,
                        )?;
                    }

                    composer.add_composable_module(shader.into())?;
                }
            }
            // if we fail to add a module the composer will tell us what is missing
        }

        Ok(())
    }

    #[allow(clippy::result_large_err)]
    fn get(
        &mut self,
        render_device: &RenderDevice,
        pipeline: CachedPipelineId,
        id: AssetId<Shader>,
        shader_defs: &[ShaderDefVal],
    ) -> Result<ErasedShaderModule, PipelineCacheError> {
        let shader = self
            .shaders
            .get(&id)
            .ok_or(PipelineCacheError::ShaderNotLoaded(id))?;
        let data = self.data.entry(id).or_default();
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
                #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
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
                    id, shader_defs
                );
                let shader_source = match &shader.source {
                    #[cfg(feature = "shader_format_spirv")]
                    Source::SpirV(data) => make_spirv(data),
                    #[cfg(not(feature = "shader_format_spirv"))]
                    Source::SpirV(_) => {
                        unimplemented!(
                            "Enable feature \"shader_format_spirv\" to use SPIR-V shaders"
                        )
                    }
                    _ => {
                        for import in shader.imports() {
                            Self::add_import_to_composer(
                                &mut self.composer,
                                &self.import_path_shaders,
                                &self.shaders,
                                import,
                            )?;
                        }

                        let shader_defs = shader_defs
                            .into_iter()
                            .chain(shader.shader_defs.iter().cloned())
                            .map(|def| match def {
                                ShaderDefVal::Bool(k, v) => {
                                    (k, naga_oil::compose::ShaderDefValue::Bool(v))
                                }
                                ShaderDefVal::Int(k, v) => {
                                    (k, naga_oil::compose::ShaderDefValue::Int(v))
                                }
                                ShaderDefVal::UInt(k, v) => {
                                    (k, naga_oil::compose::ShaderDefValue::UInt(v))
                                }
                            })
                            .collect::<std::collections::HashMap<_, _>>();

                        let naga = self.composer.make_naga_module(
                            naga_oil::compose::NagaModuleDescriptor {
                                shader_defs,
                                ..shader.into()
                            },
                        )?;

                        wgpu::ShaderSource::Naga(Cow::Owned(naga))
                    }
                };

                let module_descriptor = ShaderModuleDescriptor {
                    label: None,
                    source: shader_source,
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

    fn clear(&mut self, id: AssetId<Shader>) -> Vec<CachedPipelineId> {
        let mut shaders_to_clear = vec![id];
        let mut pipelines_to_queue = Vec::new();
        while let Some(handle) = shaders_to_clear.pop() {
            if let Some(data) = self.data.get_mut(&handle) {
                data.processed_shaders.clear();
                pipelines_to_queue.extend(data.pipelines.iter().copied());
                shaders_to_clear.extend(data.dependents.iter().copied());

                if let Some(Shader { import_path, .. }) = self.shaders.get(&handle) {
                    self.composer
                        .remove_composable_module(&import_path.module_name());
                }
            }
        }

        pipelines_to_queue
    }

    fn set_shader(&mut self, id: AssetId<Shader>, shader: Shader) -> Vec<CachedPipelineId> {
        let pipelines_to_queue = self.clear(id);
        let path = shader.import_path();
        self.import_path_shaders.insert(path.clone(), id);
        if let Some(waiting_shaders) = self.waiting_on_import.get_mut(path) {
            for waiting_shader in waiting_shaders.drain(..) {
                // resolve waiting shader import
                let data = self.data.entry(waiting_shader).or_default();
                data.resolved_imports.insert(path.clone(), id);
                // add waiting shader as dependent of this shader
                let data = self.data.entry(id).or_default();
                data.dependents.insert(waiting_shader);
            }
        }

        for import in shader.imports() {
            if let Some(import_id) = self.import_path_shaders.get(import).copied() {
                // resolve import because it is currently available
                let data = self.data.entry(id).or_default();
                data.resolved_imports.insert(import.clone(), import_id);
                // add this shader as a dependent of the import
                let data = self.data.entry(import_id).or_default();
                data.dependents.insert(id);
            } else {
                let waiting = self.waiting_on_import.entry(import.clone()).or_default();
                waiting.push(id);
            }
        }

        self.shaders.insert(id, shader);
        pipelines_to_queue
    }

    fn remove(&mut self, id: AssetId<Shader>) -> Vec<CachedPipelineId> {
        let pipelines_to_queue = self.clear(id);
        if let Some(shader) = self.shaders.remove(&id) {
            self.import_path_shaders.remove(shader.import_path());
        }

        pipelines_to_queue
    }
}

type LayoutCacheKey = (Vec<BindGroupLayoutId>, Vec<PushConstantRange>);
#[derive(Default)]
struct LayoutCache {
    layouts: HashMap<LayoutCacheKey, ErasedPipelineLayout>,
}

impl LayoutCache {
    fn get(
        &mut self,
        render_device: &RenderDevice,
        bind_group_layouts: &[BindGroupLayout],
        push_constant_ranges: Vec<PushConstantRange>,
    ) -> ErasedPipelineLayout {
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
            .clone()
    }
}

/// Cache for render and compute pipelines.
///
/// The cache stores existing render and compute pipelines allocated on the GPU, as well as
/// pending creation. Pipelines inserted into the cache are identified by a unique ID, which
/// can be used to retrieve the actual GPU object once it's ready. The creation of the GPU
/// pipeline object is deferred to the [`RenderSet::Render`] step, just before the render
/// graph starts being processed, as this requires access to the GPU.
///
/// Note that the cache does not perform automatic deduplication of identical pipelines. It is
/// up to the user not to insert the same pipeline twice to avoid wasting GPU resources.
///
/// [`RenderSet::Render`]: crate::RenderSet::Render
#[derive(Resource)]
pub struct PipelineCache {
    layout_cache: Arc<Mutex<LayoutCache>>,
    shader_cache: Arc<Mutex<ShaderCache>>,
    device: RenderDevice,
    pipelines: Vec<CachedPipeline>,
    waiting_pipelines: HashSet<CachedPipelineId>,
    new_pipelines: Mutex<Vec<CachedPipeline>>,
    /// If `true`, disables asynchronous pipeline compilation.
    /// This has no effect on MacOS, wasm, or without the `multi_threaded` feature.
    synchronous_pipeline_compilation: bool,
}

impl PipelineCache {
    pub fn pipelines(&self) -> impl Iterator<Item = &CachedPipeline> {
        self.pipelines.iter()
    }

    /// Create a new pipeline cache associated with the given render device.
    pub fn new(device: RenderDevice, synchronous_pipeline_compilation: bool) -> Self {
        Self {
            shader_cache: Arc::new(Mutex::new(ShaderCache::new(&device))),
            device,
            layout_cache: default(),
            waiting_pipelines: default(),
            new_pipelines: default(),
            pipelines: default(),
            synchronous_pipeline_compilation,
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

    fn set_shader(&mut self, id: AssetId<Shader>, shader: &Shader) {
        let mut shader_cache = self.shader_cache.lock().unwrap();
        let pipelines_to_queue = shader_cache.set_shader(id, shader.clone());
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
                        fragment.entry_point.deref(),
                        fragment.targets.as_slice(),
                    )
                });

                let descriptor = RawRenderPipelineDescriptor {
                    multiview: None,
                    depth_stencil: descriptor.depth_stencil.clone(),
                    label: descriptor.label.as_deref(),
                    layout: layout.as_deref(),
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
                    layout: layout.as_deref(),
                    module: &compute_module,
                    entry_point: &descriptor.entry_point,
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
    /// This is generally called automatically during the [`RenderSet::Render`] step, but can
    /// be called manually to force creation at a different time.
    ///
    /// [`RenderSet::Render`]: crate::RenderSet::Render
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

            CachedPipelineState::Creating(ref mut task) => {
                match bevy_utils::futures::check_ready(task) {
                    Some(Ok(pipeline)) => {
                        cached_pipeline.state = CachedPipelineState::Ok(pipeline);
                        return;
                    }
                    Some(Err(err)) => cached_pipeline.state = CachedPipelineState::Err(err),
                    _ => (),
                }
            }

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
                    error!("failed to process shader:\n{}", error_detail);
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
            #[allow(clippy::match_same_arms)]
            match event {
                // PERF: Instead of blocking waiting for the shader cache lock, try again next frame if the lock is currently held
                AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                    if let Some(shader) = shaders.get(*id) {
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

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "macos"),
    feature = "multi-threaded"
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
    not(feature = "multi-threaded")
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

/// Type of error returned by a [`PipelineCache`] when the creation of a GPU pipeline object failed.
#[derive(Error, Debug)]
pub enum PipelineCacheError {
    #[error(
        "Pipeline could not be compiled because the following shader is not loaded yet: {0:?}"
    )]
    ShaderNotLoaded(AssetId<Shader>),
    #[error(transparent)]
    ProcessShaderError(#[from] naga_oil::compose::ComposerError),
    #[error("Shader import not yet available.")]
    ShaderImportNotYetAvailable,
    #[error("Could not create shader module: {0}")]
    CreateShaderModule(String),
}
