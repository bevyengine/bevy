use crate::{
    render, AlphaMode, DrawMesh, DrawPrepass, EnvironmentMapLight, MeshPipeline, MeshPipelineKey,
    MeshUniform, PrepassPipelinePlugin, PrepassPlugin, RenderLightSystems, SetMeshBindGroup,
    SetMeshViewBindGroup, Shadow,
};
use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, AssetEvent, AssetServer, Assets, Handle};
use bevy_core_pipeline::{
    core_3d::{AlphaMask3d, Opaque3d, Transparent3d},
    prepass::NormalPrepass,
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::*,
    system::{
        lifetimeless::{Read, SRes},
        SystemParamItem,
    },
};
use bevy_reflect::TypeUuid;
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    mesh::{Mesh, MeshVertexBufferLayout},
    prelude::Image,
    render_asset::{PrepareAssetSet, RenderAssets},
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
        RenderPhase, SetItemPipeline, TrackedRenderPass,
    },
    render_resource::{
        AsBindGroup, AsBindGroupError, BindGroup, BindGroupLayout, OwnedBindingResource,
        PipelineCache, RenderPipelineDescriptor, Shader, ShaderRef, SpecializedMeshPipeline,
        SpecializedMeshPipelineError, SpecializedMeshPipelines,
    },
    renderer::RenderDevice,
    texture::FallbackImage,
    view::{ExtractedView, Msaa, VisibleEntities},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_utils::{tracing::error, HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

/// Materials are used alongside [`MaterialPlugin`] and [`MaterialMeshBundle`](crate::MaterialMeshBundle)
/// to spawn entities that are rendered with a specific [`Material`] type. They serve as an easy to use high level
/// way to render [`Mesh`] entities with custom shader logic.
///
/// Materials must implement [`AsBindGroup`] to define how data will be transferred to the GPU and bound in shaders.
/// [`AsBindGroup`] can be derived, which makes generating bindings straightforward. See the [`AsBindGroup`] docs for details.
///
/// Materials must also implement [`TypeUuid`] so they can be treated as an [`Asset`](bevy_asset::Asset).
///
/// # Example
///
/// Here is a simple Material implementation. The [`AsBindGroup`] derive has many features. To see what else is available,
/// check out the [`AsBindGroup`] documentation.
/// ```
/// # use bevy_pbr::{Material, MaterialMeshBundle};
/// # use bevy_ecs::prelude::*;
/// # use bevy_reflect::TypeUuid;
/// # use bevy_render::{render_resource::{AsBindGroup, ShaderRef}, texture::Image, color::Color};
/// # use bevy_asset::{Handle, AssetServer, Assets};
///
/// #[derive(AsBindGroup, TypeUuid, Debug, Clone)]
/// #[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e0"]
/// pub struct CustomMaterial {
///     // Uniform bindings must implement `ShaderType`, which will be used to convert the value to
///     // its shader-compatible equivalent. Most core math types already implement `ShaderType`.
///     #[uniform(0)]
///     color: Color,
///     // Images can be bound as textures in shaders. If the Image's sampler is also needed, just
///     // add the sampler attribute with a different binding index.
///     #[texture(1)]
///     #[sampler(2)]
///     color_texture: Handle<Image>,
/// }
///
/// // All functions on `Material` have default impls. You only need to implement the
/// // functions that are relevant for your material.
/// impl Material for CustomMaterial {
///     fn fragment_shader() -> ShaderRef {
///         "shaders/custom_material.wgsl".into()
///     }
/// }
///
/// // Spawn an entity using `CustomMaterial`.
/// fn setup(mut commands: Commands, mut materials: ResMut<Assets<CustomMaterial>>, asset_server: Res<AssetServer>) {
///     commands.spawn(MaterialMeshBundle {
///         material: materials.add(CustomMaterial {
///             color: Color::RED,
///             color_texture: asset_server.load("some_image.png"),
///         }),
///         ..Default::default()
///     });
/// }
/// ```
/// In WGSL shaders, the material's binding would look like this:
///
/// ```wgsl
/// @group(1) @binding(0)
/// var<uniform> color: vec4<f32>;
/// @group(1) @binding(1)
/// var color_texture: texture_2d<f32>;
/// @group(1) @binding(2)
/// var color_sampler: sampler;
/// ```
pub trait Material: AsBindGroup + Send + Sync + Clone + TypeUuid + Sized + 'static {
    /// Returns this material's vertex shader. If [`ShaderRef::Default`] is returned, the default mesh vertex shader
    /// will be used.
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's fragment shader. If [`ShaderRef::Default`] is returned, the default mesh fragment shader
    /// will be used.
    #[allow(unused_variables)]
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's [`AlphaMode`]. Defaults to [`AlphaMode::Opaque`].
    #[inline]
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }

    #[inline]
    /// Add a bias to the view depth of the mesh which can be used to force a specific render order
    /// for meshes with similar depth, to avoid z-fighting.
    /// The bias is in depth-texture units so large values may be needed to overcome small depth differences.
    fn depth_bias(&self) -> f32 {
        0.0
    }

    /// Returns this material's prepass vertex shader. If [`ShaderRef::Default`] is returned, the default prepass vertex shader
    /// will be used.
    fn prepass_vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's prepass fragment shader. If [`ShaderRef::Default`] is returned, the default prepass fragment shader
    /// will be used.
    #[allow(unused_variables)]
    fn prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Customizes the default [`RenderPipelineDescriptor`] for a specific entity using the entity's
    /// [`MaterialPipelineKey`] and [`MeshVertexBufferLayout`] as input.
    #[allow(unused_variables)]
    #[inline]
    fn specialize(
        pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(())
    }
}

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given [`Material`]
/// asset type.
pub struct MaterialPlugin<M: Material> {
    /// Controls if the prepass is enabled for the Material.
    /// For more information about what a prepass is, see the [`bevy_core_pipeline::prepass`] docs.
    ///
    /// When it is enabled, it will automatically add the [`PrepassPlugin`]
    /// required to make the prepass work on this Material.
    pub prepass_enabled: bool,
    pub _marker: PhantomData<M>,
}

impl<M: Material> Default for MaterialPlugin<M> {
    fn default() -> Self {
        Self {
            prepass_enabled: true,
            _marker: Default::default(),
        }
    }
}

impl<M: Material> Plugin for MaterialPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        app.add_asset::<M>()
            .add_plugin(ExtractComponentPlugin::<Handle<M>>::extract_visible());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<DrawFunctions<Shadow>>()
                .add_render_command::<Shadow, DrawPrepass<M>>()
                .add_render_command::<Transparent3d, DrawMaterial<M>>()
                .add_render_command::<Opaque3d, DrawMaterial<M>>()
                .add_render_command::<AlphaMask3d, DrawMaterial<M>>()
                .init_resource::<ExtractedMaterials<M>>()
                .init_resource::<RenderMaterials<M>>()
                .init_resource::<SpecializedMeshPipelines<MaterialPipeline<M>>>()
                .add_systems(ExtractSchedule, extract_materials::<M>)
                .add_systems(
                    Render,
                    (
                        prepare_materials::<M>
                            .in_set(RenderSet::Prepare)
                            .after(PrepareAssetSet::PreAssetPrepare),
                        render::queue_shadows::<M>.in_set(RenderLightSystems::QueueShadows),
                        queue_material_meshes::<M>.in_set(RenderSet::Queue),
                    ),
                );
        }

        // PrepassPipelinePlugin is required for shadow mapping and the optional PrepassPlugin
        app.add_plugin(PrepassPipelinePlugin::<M>::default());

        if self.prepass_enabled {
            app.add_plugin(PrepassPlugin::<M>::default());
        }
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<MaterialPipeline<M>>();
        }
    }
}

/// A key uniquely identifying a specialized [`MaterialPipeline`].
pub struct MaterialPipelineKey<M: Material> {
    pub mesh_key: MeshPipelineKey,
    pub bind_group_data: M::Data,
}

impl<M: Material> Eq for MaterialPipelineKey<M> where M::Data: PartialEq {}

impl<M: Material> PartialEq for MaterialPipelineKey<M>
where
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.mesh_key == other.mesh_key && self.bind_group_data == other.bind_group_data
    }
}

impl<M: Material> Clone for MaterialPipelineKey<M>
where
    M::Data: Clone,
{
    fn clone(&self) -> Self {
        Self {
            mesh_key: self.mesh_key,
            bind_group_data: self.bind_group_data.clone(),
        }
    }
}

impl<M: Material> Hash for MaterialPipelineKey<M>
where
    M::Data: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.mesh_key.hash(state);
        self.bind_group_data.hash(state);
    }
}

/// Render pipeline data for a given [`Material`].
#[derive(Resource)]
pub struct MaterialPipeline<M: Material> {
    pub mesh_pipeline: MeshPipeline,
    pub material_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    marker: PhantomData<M>,
}

impl<M: Material> Clone for MaterialPipeline<M> {
    fn clone(&self) -> Self {
        Self {
            mesh_pipeline: self.mesh_pipeline.clone(),
            material_layout: self.material_layout.clone(),
            vertex_shader: self.vertex_shader.clone(),
            fragment_shader: self.fragment_shader.clone(),
            marker: PhantomData,
        }
    }
}

impl<M: Material> SpecializedMeshPipeline for MaterialPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = MaterialPipelineKey<M>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key.mesh_key, layout)?;
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }

        descriptor.layout.insert(1, self.material_layout.clone());

        M::specialize(self, &mut descriptor, layout, key)?;
        Ok(descriptor)
    }
}

impl<M: Material> FromWorld for MaterialPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let render_device = world.resource::<RenderDevice>();

        MaterialPipeline {
            mesh_pipeline: world.resource::<MeshPipeline>().clone(),
            material_layout: M::bind_group_layout(render_device),
            vertex_shader: match M::vertex_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            fragment_shader: match M::fragment_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            marker: PhantomData,
        }
    }
}

type DrawMaterial<M> = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<M, 1>,
    SetMeshBindGroup<2>,
    DrawMesh,
);

/// Sets the bind group for a given [`Material`] at the configured `I` index.
pub struct SetMaterialBindGroup<M: Material, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: Material, const I: usize> RenderCommand<P> for SetMaterialBindGroup<M, I> {
    type Param = SRes<RenderMaterials<M>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<Handle<M>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        material_handle: &'_ Handle<M>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material = materials.into_inner().get(material_handle).unwrap();
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_material_meshes<M: Material>(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    material_pipeline: Res<MaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<MaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderMaterials<M>>,
    material_meshes: Query<(&Handle<M>, &Handle<Mesh>, &MeshUniform)>,
    images: Res<RenderAssets<Image>>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        Option<&Tonemapping>,
        Option<&DebandDither>,
        Option<&EnvironmentMapLight>,
        Option<&NormalPrepass>,
        &mut RenderPhase<Opaque3d>,
        &mut RenderPhase<AlphaMask3d>,
        &mut RenderPhase<Transparent3d>,
    )>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    for (
        view,
        visible_entities,
        tonemapping,
        dither,
        environment_map,
        normal_prepass,
        mut opaque_phase,
        mut alpha_mask_phase,
        mut transparent_phase,
    ) in &mut views
    {
        let draw_opaque_pbr = opaque_draw_functions.read().id::<DrawMaterial<M>>();
        let draw_alpha_mask_pbr = alpha_mask_draw_functions.read().id::<DrawMaterial<M>>();
        let draw_transparent_pbr = transparent_draw_functions.read().id::<DrawMaterial<M>>();

        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass.is_some() {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }

        let environment_map_loaded = match environment_map {
            Some(environment_map) => environment_map.is_loaded(&images),
            None => false,
        };
        if environment_map_loaded {
            view_key |= MeshPipelineKey::ENVIRONMENT_MAP;
        }

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= MeshPipelineKey::TONEMAP_IN_SHADER;
                view_key |= match tonemapping {
                    Tonemapping::None => MeshPipelineKey::TONEMAP_METHOD_NONE,
                    Tonemapping::Reinhard => MeshPipelineKey::TONEMAP_METHOD_REINHARD,
                    Tonemapping::ReinhardLuminance => {
                        MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE
                    }
                    Tonemapping::AcesFitted => MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED,
                    Tonemapping::AgX => MeshPipelineKey::TONEMAP_METHOD_AGX,
                    Tonemapping::SomewhatBoringDisplayTransform => {
                        MeshPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
                    }
                    Tonemapping::TonyMcMapface => MeshPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE,
                    Tonemapping::BlenderFilmic => MeshPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC,
                };
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= MeshPipelineKey::DEBAND_DITHER;
            }
        }

        let rangefinder = view.rangefinder3d();
        for visible_entity in &visible_entities.entities {
            if let Ok((material_handle, mesh_handle, mesh_uniform)) =
                material_meshes.get(*visible_entity)
            {
                if let (Some(mesh), Some(material)) = (
                    render_meshes.get(mesh_handle),
                    render_materials.get(material_handle),
                ) {
                    let mut mesh_key =
                        MeshPipelineKey::from_primitive_topology(mesh.primitive_topology)
                            | view_key;
                    match material.properties.alpha_mode {
                        AlphaMode::Blend => {
                            mesh_key |= MeshPipelineKey::BLEND_ALPHA;
                        }
                        AlphaMode::Premultiplied | AlphaMode::Add => {
                            // Premultiplied and Add share the same pipeline key
                            // They're made distinct in the PBR shader, via `premultiply_alpha()`
                            mesh_key |= MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA;
                        }
                        AlphaMode::Multiply => {
                            mesh_key |= MeshPipelineKey::BLEND_MULTIPLY;
                        }
                        _ => (),
                    }

                    let pipeline_id = pipelines.specialize(
                        &pipeline_cache,
                        &material_pipeline,
                        MaterialPipelineKey {
                            mesh_key,
                            bind_group_data: material.key.clone(),
                        },
                        &mesh.layout,
                    );
                    let pipeline_id = match pipeline_id {
                        Ok(id) => id,
                        Err(err) => {
                            error!("{}", err);
                            continue;
                        }
                    };

                    let distance = rangefinder.distance(&mesh_uniform.transform)
                        + material.properties.depth_bias;
                    match material.properties.alpha_mode {
                        AlphaMode::Opaque => {
                            opaque_phase.add(Opaque3d {
                                entity: *visible_entity,
                                draw_function: draw_opaque_pbr,
                                pipeline: pipeline_id,
                                distance,
                            });
                        }
                        AlphaMode::Mask(_) => {
                            alpha_mask_phase.add(AlphaMask3d {
                                entity: *visible_entity,
                                draw_function: draw_alpha_mask_pbr,
                                pipeline: pipeline_id,
                                distance,
                            });
                        }
                        AlphaMode::Blend
                        | AlphaMode::Premultiplied
                        | AlphaMode::Add
                        | AlphaMode::Multiply => {
                            transparent_phase.add(Transparent3d {
                                entity: *visible_entity,
                                draw_function: draw_transparent_pbr,
                                pipeline: pipeline_id,
                                distance,
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Common [`Material`] properties, calculated for a specific material instance.
pub struct MaterialProperties {
    /// The [`AlphaMode`] of this material.
    pub alpha_mode: AlphaMode,
    /// Add a bias to the view depth of the mesh which can be used to force a specific render order
    /// for meshes with equal depth, to avoid z-fighting.
    /// The bias is in depth-texture units so large values may be needed to overcome small depth differences.
    pub depth_bias: f32,
}

/// Data prepared for a [`Material`] instance.
pub struct PreparedMaterial<T: Material> {
    pub bindings: Vec<OwnedBindingResource>,
    pub bind_group: BindGroup,
    pub key: T::Data,
    pub properties: MaterialProperties,
}

#[derive(Resource)]
pub struct ExtractedMaterials<M: Material> {
    extracted: Vec<(Handle<M>, M)>,
    removed: Vec<Handle<M>>,
}

impl<M: Material> Default for ExtractedMaterials<M> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
        }
    }
}

/// Stores all prepared representations of [`Material`] assets for as long as they exist.
#[derive(Resource, Deref, DerefMut)]
pub struct RenderMaterials<T: Material>(pub HashMap<Handle<T>, PreparedMaterial<T>>);

impl<T: Material> Default for RenderMaterials<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// This system extracts all created or modified assets of the corresponding [`Material`] type
/// into the "render world".
pub fn extract_materials<M: Material>(
    mut commands: Commands,
    mut events: Extract<EventReader<AssetEvent<M>>>,
    assets: Extract<Res<Assets<M>>>,
) {
    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                changed_assets.insert(handle.clone_weak());
            }
            AssetEvent::Removed { handle } => {
                changed_assets.remove(handle);
                removed.push(handle.clone_weak());
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for handle in changed_assets.drain() {
        if let Some(asset) = assets.get(&handle) {
            extracted_assets.push((handle, asset.clone()));
        }
    }

    commands.insert_resource(ExtractedMaterials {
        extracted: extracted_assets,
        removed,
    });
}

/// All [`Material`] values of a given type that should be prepared next frame.
pub struct PrepareNextFrameMaterials<M: Material> {
    assets: Vec<(Handle<M>, M)>,
}

impl<M: Material> Default for PrepareNextFrameMaterials<M> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}

/// This system prepares all assets of the corresponding [`Material`] type
/// which where extracted this frame for the GPU.
pub fn prepare_materials<M: Material>(
    mut prepare_next_frame: Local<PrepareNextFrameMaterials<M>>,
    mut extracted_assets: ResMut<ExtractedMaterials<M>>,
    mut render_materials: ResMut<RenderMaterials<M>>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    pipeline: Res<MaterialPipeline<M>>,
) {
    let queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (handle, material) in queued_assets.into_iter() {
        match prepare_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(handle, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((handle, material));
            }
        }
    }

    for removed in std::mem::take(&mut extracted_assets.removed) {
        render_materials.remove(&removed);
    }

    for (handle, material) in std::mem::take(&mut extracted_assets.extracted) {
        match prepare_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(handle, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((handle, material));
            }
        }
    }
}

fn prepare_material<M: Material>(
    material: &M,
    render_device: &RenderDevice,
    images: &RenderAssets<Image>,
    fallback_image: &FallbackImage,
    pipeline: &MaterialPipeline<M>,
) -> Result<PreparedMaterial<M>, AsBindGroupError> {
    let prepared = material.as_bind_group(
        &pipeline.material_layout,
        render_device,
        images,
        fallback_image,
    )?;
    Ok(PreparedMaterial {
        bindings: prepared.bindings,
        bind_group: prepared.bind_group,
        key: prepared.data,
        properties: MaterialProperties {
            alpha_mode: material.alpha_mode(),
            depth_bias: material.depth_bias(),
        },
    })
}
