use self::{irradiance_volume::IrradianceVolume, prelude::EnvironmentMapLight};
#[cfg(feature = "meshlet")]
use crate::meshlet::{
    prepare_material_meshlet_meshes_main_opaque_pass, queue_material_meshlet_meshes,
    InstanceManager,
};
use crate::*;
use bevy_asset::{Asset, AssetEvent, AssetId, AssetServer};
use bevy_core_pipeline::{
    core_3d::{
        AlphaMask3d, Camera3d, Opaque3d, Opaque3dBinKey, ScreenSpaceTransmissionQuality,
        Transmissive3d, Transparent3d,
    },
    oit::OrderIndependentTransparencySettings,
    prepass::{
        DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass, OpaqueNoLightmap3dBinKey,
    },
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::entity::EntityHashSet;
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::SRes, SystemParamItem},
};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_render::extract_instances::ExtractedInstances;
use bevy_render::mesh::Mesh;
use bevy_render::render_asset::ChangedAssets;
use bevy_render::sync_world::{MainEntity, MainEntityHashMap, MainEntityHashSet, RenderEntity};
use bevy_render::view::{RenderVisibleEntities, VisibleEntities};
use bevy_render::{
    camera::TemporalJitter,
    extract_resource::ExtractResource,
    mesh::{Mesh3d, MeshVertexBufferLayoutRef, RenderMesh},
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_phase::*,
    render_resource::*,
    renderer::RenderDevice,
    view::{ExtractedView, Msaa, RenderVisibilityRanges, ViewVisibility},
    Extract,
};
use bevy_utils::tracing::error;
use bevy_utils::{HashMap, HashSet};
use core::{
    hash::Hash,
    marker::PhantomData,
    num::NonZero,
    sync::atomic::{AtomicU32, Ordering},
};

/// Materials are used alongside [`MaterialPlugin`], [`Mesh3d`], and [`MeshMaterial3d`]
/// to spawn entities that are rendered with a specific [`Material`] type. They serve as an easy to use high level
/// way to render [`Mesh3d`] entities with custom shader logic.
///
/// Materials must implement [`AsBindGroup`] to define how data will be transferred to the GPU and bound in shaders.
/// [`AsBindGroup`] can be derived, which makes generating bindings straightforward. See the [`AsBindGroup`] docs for details.
///
/// # Example
///
/// Here is a simple [`Material`] implementation. The [`AsBindGroup`] derive has many features. To see what else is available,
/// check out the [`AsBindGroup`] documentation.
///
/// ```
/// # use bevy_pbr::{Material, MeshMaterial3d};
/// # use bevy_ecs::prelude::*;
/// # use bevy_image::Image;
/// # use bevy_reflect::TypePath;
/// # use bevy_render::{mesh::{Mesh, Mesh3d}, render_resource::{AsBindGroup, ShaderRef}};
/// # use bevy_color::LinearRgba;
/// # use bevy_color::palettes::basic::RED;
/// # use bevy_asset::{Handle, AssetServer, Assets, Asset};
/// # use bevy_math::primitives::Capsule3d;
/// #
/// #[derive(AsBindGroup, Debug, Clone, Asset, TypePath)]
/// pub struct CustomMaterial {
///     // Uniform bindings must implement `ShaderType`, which will be used to convert the value to
///     // its shader-compatible equivalent. Most core math types already implement `ShaderType`.
///     #[uniform(0)]
///     color: LinearRgba,
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
/// // Spawn an entity with a mesh using `CustomMaterial`.
/// fn setup(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<CustomMaterial>>,
///     asset_server: Res<AssetServer>
/// ) {
///     commands.spawn((
///         Mesh3d(meshes.add(Capsule3d::default())),
///         MeshMaterial3d(materials.add(CustomMaterial {
///             color: RED.into(),
///             color_texture: asset_server.load("some_image.png"),
///         })),
///     ));
/// }
/// ```
///
/// In WGSL shaders, the material's binding would look like this:
///
/// ```wgsl
/// @group(2) @binding(0) var<uniform> color: vec4<f32>;
/// @group(2) @binding(1) var color_texture: texture_2d<f32>;
/// @group(2) @binding(2) var color_sampler: sampler;
/// ```
pub trait Material: Asset + AsBindGroup + Clone + Sized {
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

    /// Returns if this material should be rendered by the deferred or forward renderer.
    /// for `AlphaMode::Opaque` or `AlphaMode::Mask` materials.
    /// If `OpaqueRendererMethod::Auto`, it will default to what is selected in the `DefaultOpaqueRendererMethod` resource.
    #[inline]
    fn opaque_render_method(&self) -> OpaqueRendererMethod {
        OpaqueRendererMethod::Forward
    }

    #[inline]
    /// Add a bias to the view depth of the mesh which can be used to force a specific render order.
    /// for meshes with similar depth, to avoid z-fighting.
    /// The bias is in depth-texture units so large values may be needed to overcome small depth differences.
    fn depth_bias(&self) -> f32 {
        0.0
    }

    #[inline]
    /// Returns whether the material would like to read from [`ViewTransmissionTexture`](bevy_core_pipeline::core_3d::ViewTransmissionTexture).
    ///
    /// This allows taking color output from the [`Opaque3d`] pass as an input, (for screen-space transmission) but requires
    /// rendering to take place in a separate [`Transmissive3d`] pass.
    fn reads_view_transmission_texture(&self) -> bool {
        false
    }

    /// Returns this material's prepass vertex shader. If [`ShaderRef::Default`] is returned, the default prepass vertex shader
    /// will be used.
    ///
    /// This is used for the various [prepasses](bevy_core_pipeline::prepass) as well as for generating the depth maps
    /// required for shadow mapping.
    fn prepass_vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's prepass fragment shader. If [`ShaderRef::Default`] is returned, the default prepass fragment shader
    /// will be used.
    ///
    /// This is used for the various [prepasses](bevy_core_pipeline::prepass) as well as for generating the depth maps
    /// required for shadow mapping.
    #[allow(unused_variables)]
    fn prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's deferred vertex shader. If [`ShaderRef::Default`] is returned, the default deferred vertex shader
    /// will be used.
    fn deferred_vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's deferred fragment shader. If [`ShaderRef::Default`] is returned, the default deferred fragment shader
    /// will be used.
    #[allow(unused_variables)]
    fn deferred_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's [`crate::meshlet::MeshletMesh`] fragment shader. If [`ShaderRef::Default`] is returned,
    /// the default meshlet mesh fragment shader will be used.
    ///
    /// This is part of an experimental feature, and is unnecessary to implement unless you are using `MeshletMesh`'s.
    ///
    /// See [`crate::meshlet::MeshletMesh`] for limitations.
    #[allow(unused_variables)]
    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's [`crate::meshlet::MeshletMesh`] prepass fragment shader. If [`ShaderRef::Default`] is returned,
    /// the default meshlet mesh prepass fragment shader will be used.
    ///
    /// This is part of an experimental feature, and is unnecessary to implement unless you are using `MeshletMesh`'s.
    ///
    /// See [`crate::meshlet::MeshletMesh`] for limitations.
    #[allow(unused_variables)]
    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's [`crate::meshlet::MeshletMesh`] deferred fragment shader. If [`ShaderRef::Default`] is returned,
    /// the default meshlet mesh deferred fragment shader will be used.
    ///
    /// This is part of an experimental feature, and is unnecessary to implement unless you are using `MeshletMesh`'s.
    ///
    /// See [`crate::meshlet::MeshletMesh`] for limitations.
    #[allow(unused_variables)]
    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_deferred_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Customizes the default [`RenderPipelineDescriptor`] for a specific entity using the entity's
    /// [`MaterialPipelineKey`] and [`MeshVertexBufferLayoutRef`] as input.
    #[allow(unused_variables)]
    #[inline]
    fn specialize(
        pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
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
    /// Controls if shadows are enabled for the Material.
    pub shadows_enabled: bool,
    pub _marker: PhantomData<M>,
}

impl<M: Material> Default for MaterialPlugin<M> {
    fn default() -> Self {
        Self {
            prepass_enabled: true,
            shadows_enabled: true,
            _marker: Default::default(),
        }
    }
}

impl<M: Material> Plugin for MaterialPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        app.init_asset::<M>()
            .init_resource::<ChangedMaterials<M>>()
            .init_resource::<MaterialEntityMap<M>>()
            .add_systems(
                PostUpdate,
                (
                    maintain_changed_materials::<M>,
                    maintain_material_entity_map::<M>.after(maintain_changed_materials::<M>),
                    check_entity_needs_specialization::<M>.after(maintain_material_entity_map::<M>),
                ),
            )
            .register_type::<MeshMaterial3d<M>>()
            .add_plugins(RenderAssetPlugin::<PreparedMaterial<M>>::default());

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<DrawFunctions<Shadow>>()
                .init_resource::<EntitiesToSpecialize<M>>()
                .init_resource::<RenderMaterialInstances<M>>()
                .add_render_command::<Shadow, DrawPrepass<M>>()
                .add_render_command::<Transmissive3d, DrawMaterial<M>>()
                .add_render_command::<Transparent3d, DrawMaterial<M>>()
                .add_render_command::<Opaque3d, DrawMaterial<M>>()
                .add_render_command::<AlphaMask3d, DrawMaterial<M>>()
                .init_resource::<SpecializedMeshPipelines<MaterialPipeline<M>>>()
                .init_resource::<SpecializedPipelineCache<M>>()
                .add_systems(ExtractSchedule, extract_mesh_materials::<M>)
                .add_systems(
                    Render,
                    (
                        specialize_pipelines::<M>
                            .in_set(RenderSet::PrepareAssets)
                            .after(prepare_assets::<PreparedMaterial<M>>)
                            .after(prepare_assets::<RenderMesh>),
                        update_mesh_material_instances::<M>.in_set(RenderSet::PrepareAssets),
                        queue_shadows::<M>
                            .in_set(RenderSet::QueueMeshes)
                            .after(specialize_pipelines::<M>),
                        queue_material_meshes::<M>
                            .in_set(RenderSet::QueueMeshes)
                            .after(specialize_pipelines::<M>)
                            .after(update_mesh_material_instances::<M>),
                    ),
                );

            if self.shadows_enabled {
                render_app.add_systems(
                    Render,
                    queue_shadows::<M>
                        .in_set(RenderSet::QueueMeshes)
                        .after(prepare_assets::<PreparedMaterial<M>>),
                );
            }

            #[cfg(feature = "meshlet")]
            render_app.add_systems(
                Render,
                queue_material_meshlet_meshes::<M>
                    .in_set(RenderSet::QueueMeshes)
                    .run_if(resource_exists::<InstanceManager>),
            );

            #[cfg(feature = "meshlet")]
            render_app.add_systems(
                Render,
                prepare_material_meshlet_meshes_main_opaque_pass::<M>
                    .in_set(RenderSet::QueueMeshes)
                    .after(prepare_assets::<PreparedMaterial<M>>)
                    .before(queue_material_meshlet_meshes::<M>)
                    .run_if(resource_exists::<InstanceManager>),
            );
        }

        if self.shadows_enabled || self.prepass_enabled {
            // PrepassPipelinePlugin is required for shadow mapping and the optional PrepassPlugin
            app.add_plugins(PrepassPipelinePlugin::<M>::default());
        }

        if self.prepass_enabled {
            app.add_plugins(PrepassPlugin::<M>::default());
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<MaterialPipeline<M>>();
        }
    }
}

pub fn maintain_changed_materials<M: Material>(
    mut events: EventReader<AssetEvent<M>>,
    mut changed_assets: ResMut<ChangedMaterials<M>>,
    mut asset_entity_map: ResMut<MaterialEntityMap<M>>,
) {
    changed_assets.clear();
    let mut removed = HashSet::new();

    for event in events.read() {
        #[allow(clippy::match_same_arms)]
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                changed_assets.insert(*id);
                removed.remove(id);
            }
            AssetEvent::Removed { id } => {
                changed_assets.remove(id);
                removed.insert(*id);
            }
            AssetEvent::Unused { .. } => {}
            AssetEvent::LoadedWithDependencies { .. } => {
                // TODO: handle this
            }
        }
    }

    for asset in removed.drain() {
        asset_entity_map.remove(&asset);
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct MaterialEntityMap<M: Material>(HashMap<AssetId<M>, EntityHashSet>);

impl<M: Material> Default for MaterialEntityMap<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub fn maintain_material_entity_map<M: Material>(
    mut asset_entity_map: ResMut<MaterialEntityMap<M>>,
    query: Query<(Entity, &MeshMaterial3d<M>), Changed<MeshMaterial3d<M>>>,
) {
    // FIXME - handle removals somehow
    for (entity, handle) in &query {
        asset_entity_map
            .entry(handle.id())
            .or_default()
            .insert(entity);
    }
}

#[derive(Resource)]
pub struct EntitiesToSpecialize<M: Material> {
    entities: MainEntityHashSet,
    marker: PhantomData<M>,
}

impl<M: Material> Default for EntitiesToSpecialize<M> {
    fn default() -> Self {
        Self {
            entities: Default::default(),
            marker: Default::default(),
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
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
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
    pub marker: PhantomData<M>,
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
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key.mesh_key, layout)?;
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }

        descriptor.layout.insert(2, self.material_layout.clone());

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
    SetMeshBindGroup<1>,
    SetMaterialBindGroup<M, 2>,
    DrawMesh,
);

/// Sets the bind group for a given [`Material`] at the configured `I` index.
pub struct SetMaterialBindGroup<M: Material, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: Material, const I: usize> RenderCommand<P> for SetMaterialBindGroup<M, I> {
    type Param = (
        SRes<RenderAssets<PreparedMaterial<M>>>,
        SRes<RenderMaterialInstances<M>>,
    );
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        (materials, material_instances): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let materials = materials.into_inner();
        let material_instances = material_instances.into_inner();

        let Some(material_asset_id) = material_instances.get(&item.main_entity()) else {
            return RenderCommandResult::Skip;
        };
        let Some(material) = materials.get(*material_asset_id) else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

/// Stores all extracted instances of a [`Material`] in the render world.
#[derive(Resource, Deref, DerefMut)]
pub struct RenderMaterialInstances<M: Material>(pub MainEntityHashMap<AssetId<M>>);

impl<M: Material> Default for RenderMaterialInstances<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub const fn alpha_mode_pipeline_key(alpha_mode: AlphaMode, msaa: &Msaa) -> MeshPipelineKey {
    match alpha_mode {
        // Premultiplied and Add share the same pipeline key
        // They're made distinct in the PBR shader, via `premultiply_alpha()`
        AlphaMode::Premultiplied | AlphaMode::Add => MeshPipelineKey::BLEND_PREMULTIPLIED_ALPHA,
        AlphaMode::Blend => MeshPipelineKey::BLEND_ALPHA,
        AlphaMode::Multiply => MeshPipelineKey::BLEND_MULTIPLY,
        AlphaMode::Mask(_) => MeshPipelineKey::MAY_DISCARD,
        AlphaMode::AlphaToCoverage => match *msaa {
            Msaa::Off => MeshPipelineKey::MAY_DISCARD,
            _ => MeshPipelineKey::BLEND_ALPHA_TO_COVERAGE,
        },
        _ => MeshPipelineKey::NONE,
    }
}

pub const fn tonemapping_pipeline_key(tonemapping: Tonemapping) -> MeshPipelineKey {
    match tonemapping {
        Tonemapping::None => MeshPipelineKey::TONEMAP_METHOD_NONE,
        Tonemapping::Reinhard => MeshPipelineKey::TONEMAP_METHOD_REINHARD,
        Tonemapping::ReinhardLuminance => MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE,
        Tonemapping::AcesFitted => MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED,
        Tonemapping::AgX => MeshPipelineKey::TONEMAP_METHOD_AGX,
        Tonemapping::SomewhatBoringDisplayTransform => {
            MeshPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
        }
        Tonemapping::TonyMcMapface => MeshPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE,
        Tonemapping::BlenderFilmic => MeshPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC,
    }
}

pub const fn screen_space_specular_transmission_pipeline_key(
    screen_space_transmissive_blur_quality: ScreenSpaceTransmissionQuality,
) -> MeshPipelineKey {
    match screen_space_transmissive_blur_quality {
        ScreenSpaceTransmissionQuality::Low => {
            MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_LOW
        }
        ScreenSpaceTransmissionQuality::Medium => {
            MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_MEDIUM
        }
        ScreenSpaceTransmissionQuality::High => {
            MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_HIGH
        }
        ScreenSpaceTransmissionQuality::Ultra => {
            MeshPipelineKey::SCREEN_SPACE_SPECULAR_TRANSMISSION_ULTRA
        }
    }
}

#[derive(Resource)]
pub struct SpecializedPipelineCache<M: Material> {
    map: HashMap<(Entity, MainEntity), CachedRenderPipelineId>,
    marker: PhantomData<M>,
}

impl<M: Material> Default for SpecializedPipelineCache<M> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            marker: Default::default(),
        }
    }
}

impl<M: Material> SpecializedPipelineCache<M> {
    #[inline]
    pub fn get(&self, key: &(Entity, MainEntity)) -> Option<CachedRenderPipelineId> {
        self.map.get(key).copied()
    }

    #[inline]
    pub fn insert(
        &mut self,
        key: (Entity, MainEntity),
        value: CachedRenderPipelineId,
    ) -> Option<CachedRenderPipelineId> {
        self.map.insert(key, value)
    }
}

#[derive(Clone, Copy)]
pub enum RenderPhaseType {
    Opaque,
    AlphaMask,
    Transmissive,
    Transparent,
}

fn extract_mesh_materials<M: Material>(
    mut material_instances: ResMut<RenderMaterialInstances<M>>,
    mut entities_to_specialize: ResMut<EntitiesToSpecialize<M>>,
    query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &MeshMaterial3d<M>,
            Has<NeedsSpecialization>,
        )>,
    >,
) {
    material_instances.clear();

    for (entity, view_visibility, material, needs_specialization) in &query {
        if view_visibility.get() {
            material_instances.insert(entity.into(), material.id());
            if needs_specialization {
                entities_to_specialize.entities.insert(entity.into());
            }
        }
    }
}

fn update_mesh_material_instances<M: Material>(
    render_material_instances: Res<RenderMaterialInstances<M>>,
    render_materials: Res<RenderAssets<PreparedMaterial<M>>>,
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transmissive_draw_functions: Res<DrawFunctions<Transmissive3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
) {
    let draw_opaque_pbr = opaque_draw_functions.read().id::<DrawMaterial<M>>();
    let draw_alpha_mask_pbr = alpha_mask_draw_functions.read().id::<DrawMaterial<M>>();
    let draw_transmissive_pbr = transmissive_draw_functions.read().id::<DrawMaterial<M>>();
    let draw_transparent_pbr = transparent_draw_functions.read().id::<DrawMaterial<M>>();

    let mut updated = 0;
    match render_mesh_instances.as_mut() {
        RenderMeshInstances::CpuBuilding(render_mesh_instances) => {
            for (entity, render_mesh_instance) in render_mesh_instances.iter_mut() {
                let Some(asset_id) = render_material_instances.get(entity) else {
                    continue;
                };
                let Some(material) = render_materials.get(*asset_id) else {
                    // dbg!("No material");
                    continue;
                };
                let material_bind_group_id = material.get_bind_group_id();
                let depth_bias = material.properties.depth_bias;
                let forward = match material.properties.render_method {
                    OpaqueRendererMethod::Forward => true,
                    OpaqueRendererMethod::Deferred => false,
                    OpaqueRendererMethod::Auto => unreachable!(),
                };
                let render_phase_type = match material.properties.alpha_mode {
                    AlphaMode::Opaque => {
                        if material.properties.reads_view_transmission_texture {
                            RenderPhaseType::Transmissive
                        } else if forward {
                            RenderPhaseType::Opaque
                        } else {
                            panic!("Invalid opaque configuration");
                        }
                    }
                    AlphaMode::Mask(_) => {
                        if material.properties.reads_view_transmission_texture {
                            RenderPhaseType::Transmissive
                        } else if forward {
                            RenderPhaseType::AlphaMask
                        } else {
                            panic!("Invalid alpha mask configuration");
                        }
                    }
                    AlphaMode::Blend
                    | AlphaMode::Premultiplied
                    | AlphaMode::Add
                    | AlphaMode::Multiply
                    | AlphaMode::AlphaToCoverage => RenderPhaseType::Transparent,
                };
                let draw_function_id = match render_phase_type {
                    RenderPhaseType::Opaque => draw_opaque_pbr,
                    RenderPhaseType::AlphaMask => draw_alpha_mask_pbr,
                    RenderPhaseType::Transmissive => draw_transmissive_pbr,
                    RenderPhaseType::Transparent => draw_transparent_pbr,
                };

                updated += 1;
                render_mesh_instance
                    .material_bind_group_id
                    .set(material_bind_group_id);
                render_mesh_instance.depth_bias = depth_bias;
                render_mesh_instance.render_phase_type = render_phase_type;
                render_mesh_instance.draw_function_id = draw_function_id;
            }
        }
        RenderMeshInstances::GpuBuilding(render_mesh_instances) => {
            for (entity, render_mesh_instance) in render_mesh_instances.iter_mut() {
                let Some(asset_id) = render_material_instances.get(entity) else {
                    continue;
                };
                let Some(material) = render_materials.get(*asset_id) else {
                    // dbg!("No material");
                    continue;
                };
                let material_bind_group_id = material.get_bind_group_id();
                let depth_bias = material.properties.depth_bias;
                let forward = match material.properties.render_method {
                    OpaqueRendererMethod::Forward => true,
                    OpaqueRendererMethod::Deferred => false,
                    OpaqueRendererMethod::Auto => unreachable!(),
                };
                let render_phase_type = match material.properties.alpha_mode {
                    AlphaMode::Opaque => {
                        if material.properties.reads_view_transmission_texture {
                            RenderPhaseType::Transmissive
                        } else if forward {
                            RenderPhaseType::Opaque
                        } else {
                            panic!("Invalid opaque configuration");
                        }
                    }
                    AlphaMode::Mask(_) => {
                        if material.properties.reads_view_transmission_texture {
                            RenderPhaseType::Transmissive
                        } else if forward {
                            RenderPhaseType::AlphaMask
                        } else {
                            panic!("Invalid alpha mask configuration");
                        }
                    }
                    AlphaMode::Blend
                    | AlphaMode::Premultiplied
                    | AlphaMode::Add
                    | AlphaMode::Multiply
                    | AlphaMode::AlphaToCoverage => RenderPhaseType::Transparent,
                };
                let draw_function_id = match render_phase_type {
                    RenderPhaseType::Opaque => draw_opaque_pbr,
                    RenderPhaseType::AlphaMask => draw_alpha_mask_pbr,
                    RenderPhaseType::Transmissive => draw_transmissive_pbr,
                    RenderPhaseType::Transparent => draw_transparent_pbr,
                };

                updated += 1;
                render_mesh_instance
                    .material_bind_group_id
                    .set(material_bind_group_id);
                render_mesh_instance.depth_bias = depth_bias;
                render_mesh_instance.render_phase_type = render_phase_type;
                render_mesh_instance.draw_function_id = draw_function_id;
            }
        }
    }
    // dbg!(updated);
}

/// For each view, iterates over all the meshes visible from that view and adds
/// them to [`BinnedRenderPhase`]s or [`SortedRenderPhase`]s as appropriate.
#[allow(clippy::too_many_arguments)]
pub fn queue_material_meshes<M: Material>(
    specialized_pipeline_cache: Res<SpecializedPipelineCache<M>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    mut alpha_mask_render_phases: ResMut<ViewBinnedRenderPhases<AlphaMask3d>>,
    mut transmissive_render_phases: ResMut<ViewSortedRenderPhases<Transmissive3d>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<(Entity, &ExtractedView, &RenderVisibleEntities)>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let mut no_mesh_instance = 0;
    let mut invalid_draw_function_id = 0;
    let mut no_cached_pipeline = 0;
    for (view_entity, view, visible_entities) in &views {
        let (
            Some(opaque_phase),
            Some(alpha_mask_phase),
            Some(transmissive_phase),
            Some(transparent_phase),
        ) = (
            opaque_render_phases.get_mut(&view_entity),
            alpha_mask_render_phases.get_mut(&view_entity),
            transmissive_render_phases.get_mut(&view_entity),
            transparent_render_phases.get_mut(&view_entity),
        )
        else {
            continue;
        };

        let rangefinder = view.rangefinder3d();
        for (render_entity, visible_entity) in visible_entities.iter::<With<Mesh3d>>() {
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                no_mesh_instance += 1;
                continue;
            };
            if mesh_instance.draw_function_id == DrawFunctionId::INVALID {
                invalid_draw_function_id += 1;
            }
            let Some(pipeline) = specialized_pipeline_cache.get(&(view_entity, *visible_entity))
            else {
                no_cached_pipeline += 1;

                continue;
            };

            match mesh_instance.render_phase_type {
                RenderPhaseType::Opaque => {
                    let bin_key = Opaque3dBinKey {
                        draw_function: mesh_instance.draw_function_id,
                        pipeline,
                        asset_id: mesh_instance.mesh_asset_id.into(),
                    };
                    opaque_phase.add(
                        bin_key,
                        (*render_entity, *visible_entity),
                        BinnedRenderPhaseType::mesh(mesh_instance.should_batch()),
                    );
                }
                RenderPhaseType::AlphaMask => {
                    let bin_key = OpaqueNoLightmap3dBinKey {
                        draw_function: mesh_instance.draw_function_id,
                        pipeline,
                        asset_id: mesh_instance.mesh_asset_id.into(),
                    };
                    alpha_mask_phase.add(
                        bin_key,
                        (*render_entity, *visible_entity),
                        BinnedRenderPhaseType::mesh(mesh_instance.should_batch()),
                    );
                }
                RenderPhaseType::Transmissive => {
                    let distance = rangefinder.distance_translation(&mesh_instance.translation)
                        + mesh_instance.depth_bias;
                    transmissive_phase.add(Transmissive3d {
                        entity: (*render_entity, *visible_entity),
                        draw_function: mesh_instance.draw_function_id,
                        pipeline,
                        distance,
                        batch_range: 0..1,
                        extra_index: PhaseItemExtraIndex::NONE,
                    });
                }
                RenderPhaseType::Transparent => {
                    let distance = rangefinder.distance_translation(&mesh_instance.translation)
                        + mesh_instance.depth_bias;
                    transparent_phase.add(Transparent3d {
                        entity: (*render_entity, *visible_entity),
                        draw_function: mesh_instance.draw_function_id,
                        pipeline,
                        distance,
                        batch_range: 0..1,
                        extra_index: PhaseItemExtraIndex::NONE,
                    });
                }
            }
        }
    }
}

/// Default render method used for opaque materials.
#[derive(Default, Resource, Clone, Debug, ExtractResource, Reflect)]
#[reflect(Resource, Default, Debug)]
pub struct DefaultOpaqueRendererMethod(OpaqueRendererMethod);

impl DefaultOpaqueRendererMethod {
    pub fn forward() -> Self {
        DefaultOpaqueRendererMethod(OpaqueRendererMethod::Forward)
    }

    pub fn deferred() -> Self {
        DefaultOpaqueRendererMethod(OpaqueRendererMethod::Deferred)
    }

    pub fn set_to_forward(&mut self) {
        self.0 = OpaqueRendererMethod::Forward;
    }

    pub fn set_to_deferred(&mut self) {
        self.0 = OpaqueRendererMethod::Deferred;
    }
}

/// Render method used for opaque materials.
///
/// The forward rendering main pass draws each mesh entity and shades it according to its
/// corresponding material and the lights that affect it. Some render features like Screen Space
/// Ambient Occlusion require running depth and normal prepasses, that are 'deferred'-like
/// prepasses over all mesh entities to populate depth and normal textures. This means that when
/// using render features that require running prepasses, multiple passes over all visible geometry
/// are required. This can be slow if there is a lot of geometry that cannot be batched into few
/// draws.
///
/// Deferred rendering runs a prepass to gather not only geometric information like depth and
/// normals, but also all the material properties like base color, emissive color, reflectance,
/// metalness, etc, and writes them into a deferred 'g-buffer' texture. The deferred main pass is
/// then a fullscreen pass that reads data from these textures and executes shading. This allows
/// for one pass over geometry, but is at the cost of not being able to use MSAA, and has heavier
/// bandwidth usage which can be unsuitable for low end mobile or other bandwidth-constrained devices.
///
/// If a material indicates `OpaqueRendererMethod::Auto`, `DefaultOpaqueRendererMethod` will be used.
#[derive(Default, Clone, Copy, Debug, PartialEq, Reflect)]
pub enum OpaqueRendererMethod {
    #[default]
    Forward,
    Deferred,
    Auto,
}

/// Common [`Material`] properties, calculated for a specific material instance.
pub struct MaterialProperties {
    /// Is this material should be rendered by the deferred renderer when.
    /// [`AlphaMode::Opaque`] or [`AlphaMode::Mask`]
    pub render_method: OpaqueRendererMethod,
    /// The [`AlphaMode`] of this material.
    pub alpha_mode: AlphaMode,
    /// The bits in the [`MeshPipelineKey`] for this material.
    ///
    /// These are precalculated so that we can just "or" them together in
    /// [`queue_material_meshes`].
    pub mesh_pipeline_key_bits: MeshPipelineKey,
    /// Add a bias to the view depth of the mesh which can be used to force a specific render order
    /// for meshes with equal depth, to avoid z-fighting.
    /// The bias is in depth-texture units so large values may be needed to overcome small depth differences.
    pub depth_bias: f32,
    /// Whether the material would like to read from [`ViewTransmissionTexture`](bevy_core_pipeline::core_3d::ViewTransmissionTexture).
    ///
    /// This allows taking color output from the [`Opaque3d`] pass as an input, (for screen-space transmission) but requires
    /// rendering to take place in a separate [`Transmissive3d`] pass.
    pub reads_view_transmission_texture: bool,
}

/// Data prepared for a [`Material`] instance.
pub struct PreparedMaterial<T: Material> {
    pub bindings: Vec<(u32, OwnedBindingResource)>,
    pub bind_group: BindGroup,
    pub key: T::Data,
    pub properties: MaterialProperties,
}

impl<M: Material> RenderAsset for PreparedMaterial<M> {
    type SourceAsset = M;

    type Param = (
        SRes<RenderDevice>,
        SRes<MaterialPipeline<M>>,
        SRes<DefaultOpaqueRendererMethod>,
        M::Param,
    );

    fn prepare_asset(
        material: Self::SourceAsset,
        (render_device, pipeline, default_opaque_render_method, ref mut material_param): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        match material.as_bind_group(&pipeline.material_layout, render_device, material_param) {
            Ok(prepared) => {
                let method = match material.opaque_render_method() {
                    OpaqueRendererMethod::Forward => OpaqueRendererMethod::Forward,
                    OpaqueRendererMethod::Deferred => OpaqueRendererMethod::Deferred,
                    OpaqueRendererMethod::Auto => default_opaque_render_method.0,
                };
                let mut mesh_pipeline_key_bits = MeshPipelineKey::empty();
                mesh_pipeline_key_bits.set(
                    MeshPipelineKey::READS_VIEW_TRANSMISSION_TEXTURE,
                    material.reads_view_transmission_texture(),
                );

                Ok(PreparedMaterial {
                    bindings: prepared.bindings,
                    bind_group: prepared.bind_group,
                    key: prepared.data,
                    properties: MaterialProperties {
                        alpha_mode: material.alpha_mode(),
                        depth_bias: material.depth_bias(),
                        reads_view_transmission_texture: mesh_pipeline_key_bits
                            .contains(MeshPipelineKey::READS_VIEW_TRANSMISSION_TEXTURE),
                        render_method: method,
                        mesh_pipeline_key_bits,
                    },
                })
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                Err(PrepareAssetError::RetryNextUpdate(material))
            }
            Err(other) => Err(PrepareAssetError::AsBindGroupError(other)),
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct ChangedMaterials<M: Material>(HashSet<AssetId<M>>);

impl<M: Material> Default for ChangedMaterials<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut)]
pub struct MaterialBindGroupId(pub Option<BindGroupId>);

impl MaterialBindGroupId {
    pub fn new(id: BindGroupId) -> Self {
        Self(Some(id))
    }
}

impl From<BindGroup> for MaterialBindGroupId {
    fn from(value: BindGroup) -> Self {
        Self::new(value.id())
    }
}

/// An atomic version of [`MaterialBindGroupId`] that can be read from and written to
/// safely from multiple threads.
#[derive(Default)]
pub struct AtomicMaterialBindGroupId(AtomicU32);

impl AtomicMaterialBindGroupId {
    /// Stores a value atomically. Uses [`Ordering::Relaxed`] so there is zero guarantee of ordering
    /// relative to other operations.
    ///
    /// See also:  [`AtomicU32::store`].
    pub fn set(&self, id: MaterialBindGroupId) {
        let id = if let Some(id) = id.0 {
            NonZero::<u32>::from(id).get()
        } else {
            0
        };
        self.0.store(id, Ordering::Relaxed);
    }

    /// Loads a value atomically. Uses [`Ordering::Relaxed`] so there is zero guarantee of ordering
    /// relative to other operations.
    ///
    /// See also:  [`AtomicU32::load`].
    pub fn get(&self) -> MaterialBindGroupId {
        MaterialBindGroupId(
            NonZero::<u32>::new(self.0.load(Ordering::Relaxed)).map(BindGroupId::from),
        )
    }
}

impl<T: Material> PreparedMaterial<T> {
    pub fn get_bind_group_id(&self) -> MaterialBindGroupId {
        MaterialBindGroupId(Some(self.bind_group.id()))
    }
}

pub fn check_entity_needs_specialization<M: Material>(
    mut commands: Commands,
    query: Query<(Entity, Ref<Mesh3d>, Ref<MeshMaterial3d<M>>), Without<VisibleEntities>>,
    changed_materials: Res<ChangedMaterials<M>>,
    material_entity_map: Res<MaterialEntityMap<M>>,
    changed_meshes: Res<ChangedAssets<Mesh>>,
    mesh_entity_map: Res<AssetEntityMap<Mesh>>,
) {
    let mut need_specialization = EntityHashSet::default();
    for (entity, mesh, material) in &query {
        if mesh.is_changed() || material.is_changed() {
            need_specialization.insert(entity);
        }
    }
    for asset in changed_materials.iter() {
        if let Some(entities) = material_entity_map.get(asset) {
            need_specialization.extend(entities.iter().copied());
        }
    }
    for asset in changed_meshes.iter() {
        if let Some(entities) = mesh_entity_map.get(asset) {
            need_specialization.extend(entities.iter().copied());
        }
    }

    if !need_specialization.is_empty() {
        dbg!(need_specialization.len());
    }
    commands.insert_or_spawn_batch(
        need_specialization
            .into_iter()
            .map(|entity| (entity, NeedsSpecialization)),
    );
}

#[allow(clippy::too_many_arguments)]
fn specialize_pipelines<M: Material>(
    mut entities_to_specialize: ResMut<EntitiesToSpecialize<M>>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    render_materials: Res<RenderAssets<PreparedMaterial<M>>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_lightmaps: Res<RenderLightmaps>,
    view_key_cache: Res<ViewKeyCache>,
    material_pipeline: Res<MaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<MaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_pipeline_cache: ResMut<SpecializedPipelineCache<M>>,
    (opaque_3d_phases, alpha_mask_3d_phases, transmissive_3d_phases, transparent_3d_phases): (
        Res<ViewBinnedRenderPhases<Opaque3d>>,
        Res<ViewBinnedRenderPhases<AlphaMask3d>>,
        Res<ViewSortedRenderPhases<Transmissive3d>>,
        Res<ViewSortedRenderPhases<Transparent3d>>,
    ),
    views: Query<(Entity, &MainEntity, &RenderVisibleEntities, &Msaa)>,
    mut specialized: Local<Vec<MainEntity>>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let mut no_material_asset_id = 0;
    let mut no_mesh_instance = 0;
    let mut no_mesh = 0;
    let mut no_material = 0;
    let mut no_view_key = 0;
    specialized.clear();
    for (view_entity, main_entity, visible_entities, msaa) in &views {
        if !opaque_3d_phases.contains_key(&view_entity)
            || !alpha_mask_3d_phases.contains_key(&view_entity)
            || !transparent_3d_phases.contains_key(&view_entity)
            || !transmissive_3d_phases.contains_key(&view_entity)
        {
            continue;
        };

        for (_, visible_entity) in visible_entities.iter::<With<Mesh3d>>() {
            if entities_to_specialize.entities.contains(visible_entity) {
                let Some(material_asset_id) = render_material_instances.get(visible_entity) else {
                    no_material_asset_id += 1;
                    continue;
                };
                let Some(mesh_instance) =
                    render_mesh_instances.render_mesh_queue_data(*visible_entity)
                else {
                    no_mesh_instance += 1;
                    continue;
                };
                let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                    no_mesh += 1;
                    continue;
                };
                let Some(material) = render_materials.get(*material_asset_id) else {
                    no_material += 1;
                    continue;
                };
                let Some(view_key) = view_key_cache.get(&main_entity.id()).copied() else {
                    no_view_key += 1;
                    continue;
                };

                let mut mesh_key = MeshPipelineKey::NONE;

                mesh_key |= MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());

                if mesh.morph_targets.is_some() {
                    mesh_key |= MeshPipelineKey::MORPH_TARGETS;
                }

                if material.properties.reads_view_transmission_texture {
                    mesh_key |= MeshPipelineKey::READS_VIEW_TRANSMISSION_TEXTURE;
                }

                mesh_key |= alpha_mode_pipeline_key(material.properties.alpha_mode, msaa);

                if render_lightmaps
                    .render_lightmaps
                    .contains_key(visible_entity)
                {
                    mesh_key |= MeshPipelineKey::LIGHTMAPPED;
                }

                let pipeline_id = pipelines.specialize(
                    &pipeline_cache,
                    &material_pipeline,
                    MaterialPipelineKey {
                        mesh_key: view_key | mesh_key,
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

                // println!("Inserting due to view change {view_entity:?} {visible_entity:?}");
                specialized.push(*visible_entity);
                specialized_pipeline_cache.insert(
                    (view_entity, *visible_entity),
                    // SpecializedPipeline {
                    pipeline_id,
                    //     view_key,
                    //     mesh_key,
                    //     material_key: material.key.clone(),
                    // },
                );
            }
        }
    }
    for entity in specialized.iter() {
        entities_to_specialize.entities.remove(entity);
    }
    if no_material_asset_id > 0 {
        dbg!(no_material_asset_id);
    }
    if no_mesh_instance > 0 {
        dbg!(no_mesh_instance);
    }
    if no_mesh > 0 {
        dbg!(no_mesh);
    }
    if no_material > 0 {
        dbg!(no_material);
    }
    if no_view_key > 0 {
        dbg!(no_view_key);
    }
}
