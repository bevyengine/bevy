use crate::shader::*;
use alloc::sync::Arc;
use bevy_asset::AssetId;
use bevy_platform::collections::{hash_map::EntryRef, HashMap, HashSet};
use core::hash::Hash;
use naga::valid::Capabilities;
use thiserror::Error;
use tracing::debug;
use wgpu_types::{DownlevelFlags, Features};

/// Source of a shader module.
///
/// The source will be parsed and validated.
///
/// Any necessary shader translation (e.g. from WGSL to SPIR-V or vice versa)
/// will be done internally by wgpu.
///
/// This type is unique to the Rust API of `wgpu`. In the WebGPU specification,
/// only WGSL source code strings are accepted.
///
/// This is roughly equivalent to `wgpu::ShaderSource`
#[cfg_attr(
    not(feature = "decoupled_naga"),
    expect(
        clippy::large_enum_variant,
        reason = "naga modules are the most common use, and are large"
    )
)]
#[derive(Clone, Debug)]
pub enum ShaderCacheSource<'a> {
    /// SPIR-V module represented as a slice of words.
    SpirV(&'a [u8]),
    /// WGSL module as a string slice.
    Wgsl(String),
    /// Naga module.
    #[cfg(not(feature = "decoupled_naga"))]
    Naga(naga::Module),
}

pub type CachedPipelineId = usize;

struct ShaderData<ShaderModule> {
    pipelines: HashSet<CachedPipelineId>,
    processed_shaders: HashMap<Box<[ShaderDefVal]>, Arc<ShaderModule>>,
    resolved_imports: HashMap<ShaderImport, AssetId<Shader>>,
    dependents: HashSet<AssetId<Shader>>,
}

impl<T> Default for ShaderData<T> {
    fn default() -> Self {
        Self {
            pipelines: Default::default(),
            processed_shaders: Default::default(),
            resolved_imports: Default::default(),
            dependents: Default::default(),
        }
    }
}

pub struct ShaderCache<ShaderModule, RenderDevice> {
    data: HashMap<AssetId<Shader>, ShaderData<ShaderModule>>,
    load_module: fn(
        &RenderDevice,
        ShaderCacheSource,
        &ValidateShader,
    ) -> Result<ShaderModule, ShaderCacheError>,
    #[cfg(feature = "shader_format_wesl")]
    module_path_to_asset_id: HashMap<wesl::syntax::ModulePath, AssetId<Shader>>,
    shaders: HashMap<AssetId<Shader>, Shader>,
    import_path_shaders: HashMap<ShaderImport, AssetId<Shader>>,
    waiting_on_import: HashMap<ShaderImport, Vec<AssetId<Shader>>>,
    pub composer: naga_oil::compose::Composer,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
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

impl<ShaderModule, RenderDevice> ShaderCache<ShaderModule, RenderDevice> {
    pub fn new(
        features: Features,
        downlevel: DownlevelFlags,
        load_module: fn(
            &RenderDevice,
            ShaderCacheSource,
            &ValidateShader,
        ) -> Result<ShaderModule, ShaderCacheError>,
    ) -> Self {
        let capabilities = get_capabilities(features, downlevel);
        #[cfg(debug_assertions)]
        let composer = naga_oil::compose::Composer::default();
        #[cfg(not(debug_assertions))]
        let composer = naga_oil::compose::Composer::non_validating();

        let composer = composer.with_capabilities(capabilities);

        Self {
            composer,
            load_module,
            data: Default::default(),
            #[cfg(feature = "shader_format_wesl")]
            module_path_to_asset_id: Default::default(),
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
    ) -> Result<(), ShaderCacheError> {
        // Early out if we've already imported this module
        if composer.contains_module(&import.module_name()) {
            return Ok(());
        }

        // Check if the import is available (this handles the recursive import case)
        let shader = import_path_shaders
            .get(import)
            .and_then(|handle| shaders.get(handle))
            .ok_or(ShaderCacheError::ShaderImportNotYetAvailable)?;

        // Recurse down to ensure all import dependencies are met
        for import in &shader.imports {
            Self::add_import_to_composer(composer, import_path_shaders, shaders, import)?;
        }

        composer
            .add_composable_module(shader.into())
            .map_err(Box::new)?;
        // if we fail to add a module the composer will tell us what is missing

        Ok(())
    }

    pub fn get(
        &mut self,
        render_device: &RenderDevice,
        pipeline: CachedPipelineId,
        id: AssetId<Shader>,
        shader_defs: &[ShaderDefVal],
    ) -> Result<Arc<ShaderModule>, ShaderCacheError> {
        let shader = self
            .shaders
            .get(&id)
            .ok_or(ShaderCacheError::ShaderNotLoaded(id))?;

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
            return Err(ShaderCacheError::ShaderImportNotYetAvailable);
        }

        data.pipelines.insert(pipeline);

        // PERF: this shader_defs clone isn't great. use raw_entry_mut when it stabilizes
        let module = match data.processed_shaders.entry_ref(shader_defs) {
            EntryRef::Occupied(entry) => entry.into_mut(),
            EntryRef::Vacant(entry) => {
                debug!(
                    "processing shader {}, with shader defs {:?}",
                    id, shader_defs
                );
                let shader_source = match &shader.source {
                    Source::SpirV(data) => ShaderCacheSource::SpirV(data.as_ref()),
                    #[cfg(feature = "shader_format_wesl")]
                    Source::Wesl(_) => {
                        if let ShaderImport::AssetPath(path) = shader.import_path() {
                            let shader_resolver =
                                ShaderResolver::new(&self.module_path_to_asset_id, &self.shaders);
                            let module_path = wesl::syntax::ModulePath::from_path(path);
                            let mut compiler_options = wesl::CompileOptions {
                                imports: true,
                                condcomp: true,
                                lower: true,
                                ..Default::default()
                            };

                            for shader_def in shader_defs {
                                match shader_def {
                                    ShaderDefVal::Bool(key, value) => {
                                        compiler_options.features.flags.insert(key.clone(), (*value).into());
                                    }
                                    _ => debug!(
                                        "ShaderDefVal::Int and ShaderDefVal::UInt are not supported in wesl",
                                    ),
                                }
                            }

                            let compiled = wesl::compile(
                                &module_path,
                                &shader_resolver,
                                &wesl::EscapeMangler,
                                &compiler_options,
                            )
                            .unwrap();

                            ShaderCacheSource::Wgsl(compiled.to_string())
                        } else {
                            panic!("Wesl shaders must be imported from a file");
                        }
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
                            .iter()
                            .chain(shader.shader_defs.iter())
                            .map(|def| match def.clone() {
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

                        let naga = self
                            .composer
                            .make_naga_module(naga_oil::compose::NagaModuleDescriptor {
                                shader_defs,
                                ..shader.into()
                            })
                            .map_err(Box::new)?;

                        #[cfg(not(feature = "decoupled_naga"))]
                        {
                            ShaderCacheSource::Naga(naga)
                        }

                        #[cfg(feature = "decoupled_naga")]
                        {
                            let mut validator = naga::valid::Validator::new(
                                naga::valid::ValidationFlags::all(),
                                self.composer.capabilities,
                            );
                            let module_info = validator.validate(&naga).unwrap();
                            let wgsl = naga::back::wgsl::write_string(
                                &naga,
                                &module_info,
                                naga::back::wgsl::WriterFlags::empty(),
                            )
                            .unwrap();
                            ShaderCacheSource::Wgsl(wgsl)
                        }
                    }
                };

                let shader_module =
                    (self.load_module)(render_device, shader_source, &shader.validate_shader)?;

                entry.insert(Arc::new(shader_module))
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

    pub fn set_shader(&mut self, id: AssetId<Shader>, shader: Shader) -> Vec<CachedPipelineId> {
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

        #[cfg(feature = "shader_format_wesl")]
        if let Source::Wesl(_) = shader.source
            && let ShaderImport::AssetPath(path) = shader.import_path()
        {
            self.module_path_to_asset_id
                .insert(wesl::syntax::ModulePath::from_path(path), id);
        }
        self.shaders.insert(id, shader);
        pipelines_to_queue
    }

    pub fn remove(&mut self, id: AssetId<Shader>) -> Vec<CachedPipelineId> {
        let pipelines_to_queue = self.clear(id);
        if let Some(shader) = self.shaders.remove(&id) {
            self.import_path_shaders.remove(shader.import_path());
        }

        pipelines_to_queue
    }
}

#[cfg(feature = "shader_format_wesl")]
pub struct ShaderResolver<'a> {
    module_path_to_asset_id: &'a HashMap<wesl::syntax::ModulePath, AssetId<Shader>>,
    shaders: &'a HashMap<AssetId<Shader>, Shader>,
}

#[cfg(feature = "shader_format_wesl")]
impl<'a> ShaderResolver<'a> {
    pub fn new(
        module_path_to_asset_id: &'a HashMap<wesl::syntax::ModulePath, AssetId<Shader>>,
        shaders: &'a HashMap<AssetId<Shader>, Shader>,
    ) -> Self {
        Self {
            module_path_to_asset_id,
            shaders,
        }
    }
}

#[cfg(feature = "shader_format_wesl")]
impl<'a> wesl::Resolver for ShaderResolver<'a> {
    fn resolve_source(
        &self,
        module_path: &wesl::syntax::ModulePath,
    ) -> Result<alloc::borrow::Cow<'_, str>, wesl::ResolveError> {
        let asset_id = self
            .module_path_to_asset_id
            .get(module_path)
            .ok_or_else(|| {
                wesl::ResolveError::ModuleNotFound(
                    module_path.clone(),
                    "Invalid asset id".to_string(),
                )
            })?;

        let shader = self.shaders.get(asset_id).unwrap();
        Ok(alloc::borrow::Cow::Borrowed(shader.source.as_str()))
    }
}

/// Type of error returned by a `PipelineCache` when the creation of a GPU pipeline object failed.
#[derive(Error, Debug)]
pub enum ShaderCacheError {
    #[error(
        "Pipeline could not be compiled because the following shader could not be loaded: {0:?}"
    )]
    ShaderNotLoaded(AssetId<Shader>),
    #[error(transparent)]
    ProcessShaderError(#[from] Box<naga_oil::compose::ComposerError>),
    #[error("Shader import not yet available.")]
    ShaderImportNotYetAvailable,
    #[error("Could not create shader module: {0}")]
    CreateShaderModule(String),
}

// TODO: This needs to be kept up to date with the capabilities in the `create_validator` function in wgpu-core
// https://github.com/gfx-rs/wgpu/blob/trunk/wgpu-core/src/device/mod.rs#L449
// We can't use the `wgpu-core` function to detect the device's capabilities because `wgpu-core` isn't included in WebGPU builds.
/// Get the device's capabilities for use in `naga_oil`.
fn get_capabilities(features: Features, downlevel: DownlevelFlags) -> Capabilities {
    let mut capabilities = Capabilities::empty();
    capabilities.set(
        Capabilities::IMMEDIATES,
        features.contains(Features::IMMEDIATES),
    );
    capabilities.set(
        Capabilities::FLOAT64,
        features.contains(Features::SHADER_F64),
    );
    capabilities.set(
        Capabilities::SHADER_FLOAT16,
        features.contains(Features::SHADER_F16),
    );
    capabilities.set(
        Capabilities::SHADER_FLOAT16_IN_FLOAT32,
        downlevel.contains(DownlevelFlags::SHADER_F16_IN_F32),
    );
    capabilities.set(
        Capabilities::PRIMITIVE_INDEX,
        features.contains(Features::SHADER_PRIMITIVE_INDEX),
    );
    capabilities.set(
        Capabilities::TEXTURE_AND_SAMPLER_BINDING_ARRAY,
        features.contains(Features::TEXTURE_BINDING_ARRAY),
    );
    capabilities.set(
        Capabilities::BUFFER_BINDING_ARRAY,
        features.contains(Features::BUFFER_BINDING_ARRAY),
    );
    capabilities.set(
        Capabilities::STORAGE_TEXTURE_BINDING_ARRAY,
        features.contains(Features::TEXTURE_BINDING_ARRAY)
            && features.contains(Features::STORAGE_RESOURCE_BINDING_ARRAY),
    );
    capabilities.set(
        Capabilities::STORAGE_BUFFER_BINDING_ARRAY,
        features.contains(Features::BUFFER_BINDING_ARRAY)
            && features.contains(Features::STORAGE_RESOURCE_BINDING_ARRAY),
    );
    capabilities.set(
        Capabilities::TEXTURE_AND_SAMPLER_BINDING_ARRAY_NON_UNIFORM_INDEXING,
        features.contains(Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING),
    );
    capabilities.set(
        Capabilities::BUFFER_BINDING_ARRAY_NON_UNIFORM_INDEXING,
        features.contains(Features::UNIFORM_BUFFER_BINDING_ARRAYS),
    );
    capabilities.set(
        Capabilities::STORAGE_TEXTURE_BINDING_ARRAY_NON_UNIFORM_INDEXING,
        features.contains(Features::STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING),
    );
    capabilities.set(
        Capabilities::STORAGE_BUFFER_BINDING_ARRAY_NON_UNIFORM_INDEXING,
        features.contains(Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING),
    );
    capabilities.set(
        Capabilities::STORAGE_TEXTURE_16BIT_NORM_FORMATS,
        features.contains(Features::TEXTURE_FORMAT_16BIT_NORM),
    );
    capabilities.set(
        Capabilities::MULTIVIEW,
        features.contains(Features::MULTIVIEW),
    );
    capabilities.set(
        Capabilities::EARLY_DEPTH_TEST,
        features.contains(Features::SHADER_EARLY_DEPTH_TEST),
    );
    capabilities.set(
        Capabilities::SHADER_INT64,
        features.contains(Features::SHADER_INT64),
    );
    capabilities.set(
        Capabilities::SHADER_INT64_ATOMIC_MIN_MAX,
        features.intersects(
            Features::SHADER_INT64_ATOMIC_MIN_MAX | Features::SHADER_INT64_ATOMIC_ALL_OPS,
        ),
    );
    capabilities.set(
        Capabilities::SHADER_INT64_ATOMIC_ALL_OPS,
        features.contains(Features::SHADER_INT64_ATOMIC_ALL_OPS),
    );
    capabilities.set(
        Capabilities::TEXTURE_ATOMIC,
        features.contains(Features::TEXTURE_ATOMIC),
    );
    capabilities.set(
        Capabilities::TEXTURE_INT64_ATOMIC,
        features.contains(Features::TEXTURE_INT64_ATOMIC),
    );
    capabilities.set(
        Capabilities::SHADER_FLOAT32_ATOMIC,
        features.contains(Features::SHADER_FLOAT32_ATOMIC),
    );
    capabilities.set(
        Capabilities::MULTISAMPLED_SHADING,
        downlevel.contains(DownlevelFlags::MULTISAMPLED_SHADING),
    );
    capabilities.set(
        Capabilities::DUAL_SOURCE_BLENDING,
        features.contains(Features::DUAL_SOURCE_BLENDING),
    );
    capabilities.set(
        Capabilities::CLIP_DISTANCE,
        features.contains(Features::CLIP_DISTANCES),
    );
    capabilities.set(
        Capabilities::CUBE_ARRAY_TEXTURES,
        downlevel.contains(DownlevelFlags::CUBE_ARRAY_TEXTURES),
    );
    capabilities.set(
        Capabilities::SUBGROUP,
        features.intersects(Features::SUBGROUP | Features::SUBGROUP_VERTEX),
    );
    capabilities.set(
        Capabilities::SUBGROUP_BARRIER,
        features.intersects(Features::SUBGROUP_BARRIER),
    );
    capabilities.set(
        Capabilities::RAY_QUERY,
        features.intersects(Features::EXPERIMENTAL_RAY_QUERY),
    );
    capabilities.set(
        Capabilities::SUBGROUP_VERTEX_STAGE,
        features.contains(Features::SUBGROUP_VERTEX),
    );
    capabilities.set(
        Capabilities::RAY_HIT_VERTEX_POSITION,
        features.intersects(Features::EXPERIMENTAL_RAY_HIT_VERTEX_RETURN),
    );
    capabilities.set(
        Capabilities::TEXTURE_EXTERNAL,
        features.intersects(Features::EXTERNAL_TEXTURE),
    );
    capabilities.set(
        Capabilities::SHADER_BARYCENTRICS,
        features.intersects(Features::SHADER_BARYCENTRICS),
    );
    capabilities.set(
        Capabilities::MESH_SHADER,
        features.intersects(Features::EXPERIMENTAL_MESH_SHADER),
    );
    capabilities.set(
        Capabilities::MESH_SHADER_POINT_TOPOLOGY,
        features.intersects(Features::EXPERIMENTAL_MESH_SHADER_POINTS),
    );

    capabilities
}
