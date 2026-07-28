use crate::{
    init_mesh_2d_pipeline, DrawMesh2d, Mesh2d, Mesh2dPipeline, Mesh2dPipelineKey,
    RenderMesh2dInstances, SetMesh2dBindGroup, SetMesh2dViewBindGroup, ViewKeyCache,
};
use alloc::sync::Arc;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::prelude::AssetChanged;
use bevy_asset::{
    AsAssetId, Asset, AssetApp, AssetEventSystems, AssetId, AssetServer, Handle, UntypedAssetId,
};
use bevy_camera::visibility::ViewVisibility;
use bevy_core_pipeline::{
    core_2d::{
        AlphaMask2d, AlphaMask2dBinKey, BatchSetKey2d, Opaque2d, Opaque2dBinKey, Transparent2d,
    },
    tonemapping::Tonemapping,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::system::{SystemParam, SystemState};
use bevy_ecs::{
    prelude::*,
    system::{
        lifetimeless::{SRes, SResMut},
        SystemParamItem,
    },
};
use bevy_material::key::{ErasedMaterialKey, ErasedMaterialPipelineKey, ErasedMeshPipelineKey};
use bevy_material::labels::{DrawFunctionLabel, InternedShaderLabel, ShaderLabel};
use bevy_material::{AlphaMode, MaterialProperties, OpaqueRendererMethod, RenderPhaseType};
use bevy_math::FloatOrd;
use bevy_mesh::MeshVertexBufferLayoutRef;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_platform::hash::FixedHasher;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::camera::{DirtySpecializationSystems, DirtySpecializations, PendingQueues};
use bevy_render::erased_render_asset::{
    ErasedRenderAsset, ErasedRenderAssetPlugin, ErasedRenderAssets, PrepareAssetError,
};
use bevy_render::material_bind_groups::{
    material_uses_bindless_resources, MaterialBindGroupAllocators, MaterialBindingId,
    RenderMaterialBindings,
};
use bevy_render::sync_world::MainEntityHashSet;
use bevy_render::view::{RenderVisibleEntities, RetainedViewEntity};
use bevy_render::{
    mesh::RenderMesh,
    render_asset::{prepare_assets, RenderAssets},
    render_phase::{
        AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, InputUniformIndex, PhaseItem,
        PhaseItemExtraIndex, RenderCommand, RenderCommandResult, SetItemPipeline,
        TrackedRenderPass, ViewBinnedRenderPhases, ViewSortedRenderPhases,
    },
    render_resource::{
        AsBindGroup, BindGroupId, CachedRenderPipelineId, PipelineCache, RenderPipelineDescriptor,
        SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
    },
    renderer::RenderDevice,
    sync_world::{MainEntity, MainEntityHashMap},
    view::ExtractedView,
    Extract, ExtractSchedule, GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::{Shader, ShaderDefVal, ShaderRef};
use bevy_utils::Parallel;
use core::{hash::Hash, marker::PhantomData};
use derive_more::derive::From;
use smallvec::SmallVec;
use std::any::{Any, TypeId};
use tracing::error;

pub const MATERIAL_2D_BIND_GROUP_INDEX: usize = 2;

/// Materials are used alongside [`Material2dPlugin`], [`Mesh2d`], and [`MeshMaterial2d`]
/// to spawn entities that are rendered with a specific [`Material2d`] type. They serve as an easy to use high level
/// way to render [`Mesh2d`] entities with custom shader logic.
///
/// Materials must implement [`AsBindGroup`] to define how data will be transferred to the GPU and bound in shaders.
/// [`AsBindGroup`] can be derived, which makes generating bindings straightforward. See the [`AsBindGroup`] docs for details.
///
/// # Example
///
/// Here is a simple [`Material2d`] implementation. The [`AsBindGroup`] derive has many features. To see what else is available,
/// check out the [`AsBindGroup`] documentation.
///
/// ```
/// # use bevy_sprite_render::{Material2d, MeshMaterial2d};
/// # use bevy_ecs::prelude::*;
/// # use bevy_image::Image;
/// # use bevy_reflect::TypePath;
/// # use bevy_mesh::{Mesh, Mesh2d};
/// # use bevy_render::render_resource::AsBindGroup;
/// # use bevy_shader::ShaderRef;
/// # use bevy_color::LinearRgba;
/// # use bevy_color::palettes::basic::RED;
/// # use bevy_asset::{Handle, AssetServer, Assets, Asset};
/// # use bevy_math::primitives::Circle;
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
/// // All functions on `Material2d` have default impls. You only need to implement the
/// // functions that are relevant for your material.
/// impl Material2d for CustomMaterial {
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
///     asset_server: Res<AssetServer>,
/// ) {
///     commands.spawn((
///         Mesh2d(meshes.add(Circle::new(50.0))),
///         MeshMaterial2d(materials.add(CustomMaterial {
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
/// struct CustomMaterial {
///     color: vec4<f32>,
/// }
///
/// @group(2) @binding(0) var<uniform> material: CustomMaterial;
/// @group(2) @binding(1) var color_texture: texture_2d<f32>;
/// @group(2) @binding(2) var color_sampler: sampler;
/// ```
pub trait Material2d: AsBindGroup + Asset + Clone + Sized {
    /// Returns this material's vertex shader. If [`ShaderRef::Default`] is returned, the default mesh vertex shader
    /// will be used.
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's fragment shader. If [`ShaderRef::Default`] is returned, the default mesh fragment shader
    /// will be used.
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Add a bias to the view depth of the mesh which can be used to force a specific render order.
    #[inline]
    fn depth_bias(&self) -> f32 {
        0.0
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Opaque
    }

    /// Customizes the default [`RenderPipelineDescriptor`].
    #[expect(
        unused_variables,
        reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
    )]
    #[inline]
    fn specialize(
        pipeline: &Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(())
    }
}

/// A [material](Material2d) used for rendering a [`Mesh2d`].
///
/// See [`Material2d`] for general information about 2D materials and how to implement your own materials.
///
/// # Example
///
/// ```
/// # use bevy_sprite_render::{ColorMaterial, MeshMaterial2d};
/// # use bevy_ecs::prelude::*;
/// # use bevy_mesh::{Mesh, Mesh2d};
/// # use bevy_color::palettes::basic::RED;
/// # use bevy_asset::Assets;
/// # use bevy_math::primitives::Circle;
/// #
/// // Spawn an entity with a mesh using `ColorMaterial`.
/// fn setup(
///     mut commands: Commands,
///     mut meshes: ResMut<Assets<Mesh>>,
///     mut materials: ResMut<Assets<ColorMaterial>>,
/// ) {
///     commands.spawn((
///         Mesh2d(meshes.add(Circle::new(50.0))),
///         MeshMaterial2d(materials.add(ColorMaterial::from_color(RED))),
///     ));
/// }
/// ```
///
/// [`MeshMaterial2d`]: crate::MeshMaterial2d
#[derive(Component, FromTemplate, Clone, Debug, Deref, DerefMut, Reflect, From)]
#[reflect(Component, Default, Clone)]
pub struct MeshMaterial2d<M: Material2d>(pub Handle<M>);

impl<M: Material2d> Default for MeshMaterial2d<M> {
    fn default() -> Self {
        Self(Handle::default())
    }
}

impl<M: Material2d> PartialEq for MeshMaterial2d<M> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<M: Material2d> Eq for MeshMaterial2d<M> {}

impl<M: Material2d> From<MeshMaterial2d<M>> for AssetId<M> {
    fn from(material: MeshMaterial2d<M>) -> Self {
        material.id()
    }
}

impl<M: Material2d> From<&MeshMaterial2d<M>> for AssetId<M> {
    fn from(material: &MeshMaterial2d<M>) -> Self {
        material.id()
    }
}

impl<M: Material2d> AsAssetId for MeshMaterial2d<M> {
    type Asset = M;

    fn as_asset_id(&self) -> AssetId<Self::Asset> {
        self.id()
    }
}

/// Sets how a 2d material's base color alpha channel is used for transparency.
/// Currently, this only works with [`Mesh2d`]. Sprites are always transparent.
///
/// This is very similar to [`AlphaMode`] but this only applies to 2d meshes.
/// We use a separate type because 2d doesn't support all the transparency modes that 3d does.
#[derive(Debug, Default, Reflect, Copy, Clone, PartialEq)]
#[reflect(Default, Debug, Clone)]
pub enum AlphaMode2d {
    /// Base color alpha values are overridden to be fully opaque (1.0).
    #[default]
    Opaque,
    /// Reduce transparency to fully opaque or fully transparent
    /// based on a threshold.
    ///
    /// Compares the base color alpha value to the specified threshold.
    /// If the value is below the threshold,
    /// considers the color to be fully transparent (alpha is set to 0.0).
    /// If it is equal to or above the threshold,
    /// considers the color to be fully opaque (alpha is set to 1.0).
    Mask(f32),
    /// The base color alpha value defines the opacity of the color.
    /// Standard alpha-blending is used to blend the fragment's color
    /// with the color behind it.
    Blend,
}

impl From<AlphaMode2d> for AlphaMode {
    fn from(alpha_mode: AlphaMode2d) -> Self {
        match alpha_mode {
            AlphaMode2d::Opaque => AlphaMode::Opaque,
            AlphaMode2d::Mask(mask) => AlphaMode::Mask(mask),
            AlphaMode2d::Blend => AlphaMode::Blend,
        }
    }
}

/// A [`Plugin`] that supplies generic infrastructure for 2D materials.
#[derive(Default)]
pub struct Materials2dPlugin;

impl Plugin for Materials2dPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_gpu_resource::<SpecializedMaterial2dPipelineCache>()
            .init_gpu_resource::<SpecializedMeshPipelines<Material2dPipelineSpecializer>>()
            .add_render_command::<Opaque2d, DrawMaterial2d>()
            .add_render_command::<AlphaMask2d, DrawMaterial2d>()
            .add_render_command::<Transparent2d, DrawMaterial2d>()
            .init_resource::<RenderMaterial2dInstances>()
            .allow_ambiguous_resource::<RenderMaterial2dInstances>()
            .add_systems(
                RenderStartup,
                init_material_2d_pipeline.after(init_mesh_2d_pipeline),
            )
            .add_systems(
                Render,
                (
                    specialize_material2d_meshes
                        .in_set(RenderSystems::Specialize)
                        .after(prepare_assets::<RenderMesh>)
                        .after(prepare_pending_mesh_material2d_queues),
                    queue_material2d_meshes.in_set(RenderSystems::QueueMeshes),
                ),
            );
    }
}

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given [`Material2d`]
/// asset type (which includes [`Material2d`] types).
pub struct Material2dPlugin<M: Material2d>(PhantomData<M>);

impl<M: Material2d> Default for Material2dPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: Material2d> Plugin for Material2dPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        app.init_asset::<M>()
            .init_resource::<EntitiesNeedingSpecialization<M>>()
            .register_type::<MeshMaterial2d<M>>()
            .add_plugins(ErasedRenderAssetPlugin::<MeshMaterial2d<M>>::default())
            .add_systems(
                PostUpdate,
                check_entities_needing_specialization::<M>.after(AssetEventSystems),
            );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            let shaders = initialize_material2d_shaders::<M>(render_app.world());
            render_app
                .insert_resource(Material2dShaders::<M>::with_shader_cache(shaders))
                .init_resource::<PendingMeshMaterial2dQueues>()
                .allow_ambiguous_resource::<PendingMeshMaterial2dQueues>()
                .add_systems(RenderStartup, add_material2d_bind_group_allocator::<M>)
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_entities_needs_specialization::<M>
                            .in_set(DirtySpecializationSystems::CheckForChanges),
                        extract_entities_that_need_specializations_removed::<M>
                            .in_set(DirtySpecializationSystems::CheckForRemovals),
                        extract_mesh_materials_2d::<M>,
                    ),
                );
        }
    }
}

/// A render app system that creates a bind group allocator for a new 2D
/// material.
fn add_material2d_bind_group_allocator<M>(
    render_device: Res<RenderDevice>,
    mut bind_group_allocators: ResMut<MaterialBindGroupAllocators>,
) where
    M: Material2d,
{
    bind_group_allocators.add::<M>(&render_device);
}

/// A resource, part of the render world, that maps each renderable main-world
/// entity to its material's asset ID.
#[derive(Default, Resource, Deref, DerefMut)]
pub struct RenderMaterial2dInstances(MainEntityHashMap<UntypedAssetId>);

pub fn extract_mesh_materials_2d<M: Material2d>(
    mut material_instances: ResMut<RenderMaterial2dInstances>,
    changed_meshes_query: Extract<
        Query<
            (Entity, &ViewVisibility, &MeshMaterial2d<M>),
            Or<(Changed<ViewVisibility>, Changed<MeshMaterial2d<M>>)>,
        >,
    >,
    mut removed_materials_query: Extract<RemovedComponents<MeshMaterial2d<M>>>,
) {
    for (entity, view_visibility, material) in &changed_meshes_query {
        if view_visibility.get() {
            add_mesh_instance(entity, material, &mut material_instances);
        } else {
            remove_mesh_instance(entity, &mut material_instances);
        }
    }

    for entity in removed_materials_query.read() {
        // Only queue a mesh for removal if we didn't pick it up above.
        // It's possible that a necessary component was removed and re-added in
        // the same frame.
        if !changed_meshes_query.contains(entity) {
            remove_mesh_instance(entity, &mut material_instances);
        }
    }

    fn add_mesh_instance<M>(
        entity: Entity,
        material: &MeshMaterial2d<M>,
        material_instances: &mut RenderMaterial2dInstances,
    ) where
        M: Material2d,
    {
        material_instances.insert(entity.into(), material.id().untyped());
    }

    fn remove_mesh_instance(entity: Entity, material_instances: &mut RenderMaterial2dInstances) {
        material_instances.remove(&MainEntity::from(entity));
    }
}

/// Render pipeline data for a given [`Material2d`]
#[derive(Resource, Clone)]
pub struct Material2dPipeline {
    pub mesh2d_pipeline: Mesh2dPipeline,
}

/// A type that implements [`SpecializedMeshPipeline`], allowing pipelines to be
/// specialized for a single material.
pub struct Material2dPipelineSpecializer {
    /// The material 2D pipeline to be specialized.
    pub(crate) pipeline: Material2dPipeline,
    /// Common material properties for that material.
    pub(crate) properties: Arc<MaterialProperties>,
}

pub struct Material2dKey<M: Material2d> {
    pub mesh_key: Mesh2dPipelineKey,
    pub bind_group_data: M::Data,
}

impl<M: Material2d> Eq for Material2dKey<M> where M::Data: PartialEq {}

impl<M: Material2d> PartialEq for Material2dKey<M>
where
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.mesh_key == other.mesh_key && self.bind_group_data == other.bind_group_data
    }
}

impl<M: Material2d> Clone for Material2dKey<M>
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

impl<M: Material2d> Hash for Material2dKey<M>
where
    M::Data: Hash,
{
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.mesh_key.hash(state);
        self.bind_group_data.hash(state);
    }
}

impl SpecializedMeshPipeline for Material2dPipelineSpecializer {
    type Key = ErasedMaterialPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let concrete_mesh_key: Mesh2dPipelineKey = key.mesh_key.downcast();
        let mut descriptor = self
            .pipeline
            .mesh2d_pipeline
            .specialize(concrete_mesh_key, layout)?;

        descriptor.vertex.shader_defs.push(ShaderDefVal::UInt(
            "MATERIAL_BIND_GROUP".into(),
            MATERIAL_2D_BIND_GROUP_INDEX as u32,
        ));
        if let Some(ref mut fragment) = descriptor.fragment {
            fragment.shader_defs.push(ShaderDefVal::UInt(
                "MATERIAL_BIND_GROUP".into(),
                MATERIAL_2D_BIND_GROUP_INDEX as u32,
            ));
        }
        if let Some(vertex_shader) = self.properties.get_shader(Material2dVertexShader) {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = self.properties.get_shader(Material2dFragmentShader) {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }

        descriptor
            .layout
            .insert(2, self.properties.material_layout.as_ref().unwrap().clone());

        if let Some(specialize) = self.properties.user_specialize {
            specialize(&self.pipeline as &dyn Any, &mut descriptor, layout, key)?;
        }

        // If bindless mode is on, add a `BINDLESS` define.
        if self.properties.bindless {
            descriptor.vertex.shader_defs.push("BINDLESS".into());
            if let Some(ref mut fragment) = descriptor.fragment {
                fragment.shader_defs.push("BINDLESS".into());
            }
        }

        Ok(descriptor)
    }
}

pub fn init_material_2d_pipeline(mut commands: Commands, mesh_2d_pipeline: Res<Mesh2dPipeline>) {
    commands.insert_resource(Material2dPipeline {
        mesh2d_pipeline: mesh_2d_pipeline.clone(),
    });
}

pub(super) type DrawMaterial2d = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetMesh2dBindGroup<1>,
    SetMaterial2dBindGroup<MATERIAL_2D_BIND_GROUP_INDEX>,
    DrawMesh2d,
);

pub struct SetMaterial2dBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMaterial2dBindGroup<I> {
    type Param = (
        SRes<ErasedRenderAssets<PreparedMaterial2d>>,
        SRes<RenderMaterial2dInstances>,
        SRes<MaterialBindGroupAllocators>,
    );
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        (materials, material_instances, material_bind_group_allocators): SystemParamItem<
            'w,
            '_,
            Self::Param,
        >,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material_bind_group_allocators = material_bind_group_allocators.into_inner();

        let materials = materials.into_inner();
        let material_instances = material_instances.into_inner();
        let Some(material_instance) = material_instances.get(&item.main_entity()) else {
            return RenderCommandResult::Skip;
        };
        let Some(material2d) = materials.get(*material_instance) else {
            return RenderCommandResult::Skip;
        };
        let Some(material_bind_group_allocator) =
            material_bind_group_allocators.get(&material_instance.type_id())
        else {
            return RenderCommandResult::Skip;
        };
        let Some(material_bind_group) = material_bind_group_allocator.get(material2d.binding.group)
        else {
            return RenderCommandResult::Skip;
        };
        let Some(bind_group) = material_bind_group.bind_group() else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, bind_group, &[]);
        RenderCommandResult::Success
    }
}

/// Returns the 2D pipeline key for the given transparent alpha mode.
///
/// If the alpha mode is opaque, or if the alpha mode isn't supported by the 2D
/// pipeline, returns [`Mesh2dPipelineKey::NONE`].
pub const fn alpha_mode_pipeline_key_2d(alpha_mode: AlphaMode) -> Mesh2dPipelineKey {
    match alpha_mode {
        AlphaMode::Blend => Mesh2dPipelineKey::BLEND_ALPHA,
        AlphaMode::Mask(_) => Mesh2dPipelineKey::MAY_DISCARD,
        _ => Mesh2dPipelineKey::NONE,
    }
}

pub const fn tonemapping_pipeline_key(tonemapping: Tonemapping) -> Mesh2dPipelineKey {
    match tonemapping {
        Tonemapping::None => Mesh2dPipelineKey::TONEMAP_METHOD_NONE,
        Tonemapping::Reinhard => Mesh2dPipelineKey::TONEMAP_METHOD_REINHARD,
        Tonemapping::ReinhardLuminance => Mesh2dPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE,
        Tonemapping::AcesFitted => Mesh2dPipelineKey::TONEMAP_METHOD_ACES_FITTED,
        Tonemapping::AgX => Mesh2dPipelineKey::TONEMAP_METHOD_AGX,
        Tonemapping::SomewhatBoringDisplayTransform => {
            Mesh2dPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
        }
        Tonemapping::TonyMcMapface => Mesh2dPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE,
        Tonemapping::BlenderFilmic => Mesh2dPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC,
        Tonemapping::KhronosPbrNeutral => Mesh2dPipelineKey::TONEMAP_METHOD_PBR_NEUTRAL,
    }
}

pub fn extract_entities_needs_specialization<M>(
    entities_needing_specialization: Extract<Res<EntitiesNeedingSpecialization<M>>>,
    mut dirty_specializations: ResMut<DirtySpecializations>,
) where
    M: Material2d,
{
    // Drain the list of entities needing specialization from the main world
    // into the render-world `DirtySpecializations` table.
    for entity in entities_needing_specialization.changed.iter() {
        dirty_specializations
            .changed_renderables
            .insert(MainEntity::from(*entity));
    }
}

/// A system that adds entities that were judged to need their specializations
/// removed to the appropriate table in [`DirtySpecializations`].
pub fn extract_entities_that_need_specializations_removed<M>(
    entities_needing_specialization: Extract<Res<EntitiesNeedingSpecialization<M>>>,
    mut dirty_specializations: ResMut<DirtySpecializations>,
) where
    M: Material2d,
{
    for entity in entities_needing_specialization.removed.iter() {
        dirty_specializations
            .removed_renderables
            .insert(MainEntity::from(*entity));
    }
}

/// Temporarily stores entities that were determined to either need their
/// specialized pipelines updated or to have their specialized pipelines
/// removed.
#[derive(Clone, Resource, Debug)]
pub struct EntitiesNeedingSpecialization<M> {
    /// Entities that need to have their pipelines updated.
    pub changed: Vec<Entity>,
    /// Entities that need to have their pipelines removed.
    pub removed: Vec<Entity>,
    _marker: PhantomData<M>,
}

impl<M> Default for EntitiesNeedingSpecialization<M> {
    fn default() -> Self {
        Self {
            changed: Default::default(),
            removed: Default::default(),
            _marker: Default::default(),
        }
    }
}

/// Stores the [`SpecializedMaterial2dViewPipelineCache`] for each view.
#[derive(Default, Resource, Deref, DerefMut)]
pub struct SpecializedMaterial2dPipelineCache {
    // view_entity -> view pipeline cache
    #[deref]
    map: HashMap<RetainedViewEntity, SpecializedMaterial2dViewPipelineCache>,
}

/// Stores the cached render pipeline ID for each entity in a single view, as
/// well as the last time it was changed.
#[derive(Default, Deref, DerefMut)]
pub struct SpecializedMaterial2dViewPipelineCache {
    // material entity -> (tick, pipeline_id)
    #[deref]
    map: MainEntityHashMap<CachedRenderPipelineId>,
}

/// Finds 2D entities that have changed in such a way as to potentially require
/// specialization and adds them to the [`EntitiesNeedingSpecialization`] list.
pub fn check_entities_needing_specialization<M>(
    needs_specialization: Query<
        Entity,
        (
            Or<(
                Changed<Mesh2d>,
                AssetChanged<Mesh2d>,
                Changed<MeshMaterial2d<M>>,
                AssetChanged<MeshMaterial2d<M>>,
            )>,
            With<MeshMaterial2d<M>>,
        ),
    >,
    mut par_local: Local<Parallel<Vec<Entity>>>,
    mut entities_needing_specialization: ResMut<EntitiesNeedingSpecialization<M>>,
    mut removed_mesh_2d_components: RemovedComponents<Mesh2d>,
    mut removed_mesh_material_2d_components: RemovedComponents<MeshMaterial2d<M>>,
) where
    M: Material2d,
{
    entities_needing_specialization.changed.clear();
    entities_needing_specialization.removed.clear();

    // Gather all entities that need their specializations regenerated.
    needs_specialization
        .par_iter()
        .for_each(|entity| par_local.borrow_local_mut().push(entity));
    par_local.drain_into(&mut entities_needing_specialization.changed);

    // All entities that removed their `Mesh2d` or `MeshMaterial2d` components
    // need to have their specializations removed as well.
    //
    // It's possible that `Mesh2d` was removed and re-added in the same frame,
    // but we don't have to handle that situation specially here, because
    // `specialize_material2d_meshes` processes specialization removals before
    // additions. So, if the pipeline specialization gets spuriously removed,
    // it'll just be immediately re-added again, which is harmless.
    for entity in removed_mesh_2d_components
        .read()
        .chain(removed_mesh_material_2d_components.read())
    {
        entities_needing_specialization.removed.push(entity);
    }
}

/// Holds all entities with 2D mesh materials that couldn't be specialized
/// and/or queued because their materials hadn't loaded yet.
///
/// See the [`PendingQueues`] documentation for more information.
#[derive(Default, Deref, DerefMut, Resource)]
pub struct PendingMeshMaterial2dQueues(pub PendingQueues);

/// Prepares the [`PendingMeshMaterial2dQueues`] for a new frame by swapping
/// the current and previous frame queues for each view.
pub fn prepare_pending_mesh_material2d_queues(
    mut pending_mesh_material2d_queues: ResMut<PendingMeshMaterial2dQueues>,
    views: Query<&ExtractedView>,
) {
    let mut all_views: HashSet<RetainedViewEntity, FixedHasher> = HashSet::default();
    for view in &views {
        all_views.insert(view.retained_view_entity);
        pending_mesh_material2d_queues.prepare_for_new_frame(view.retained_view_entity);
    }
    pending_mesh_material2d_queues.expire_stale_views(&all_views);
}

/// The system parameter that [`specialize_material2d_meshes`] uses.
///
/// This has to be declared separately because [`specialize_material2d_meshes`]
/// takes the [`World`] directly.
#[derive(SystemParam)]
pub struct SpecializeMaterial2dMeshesSystemParam<'w, 's> {
    render_meshes: Res<'w, RenderAssets<RenderMesh>>,
    render_materials: Res<'w, ErasedRenderAssets<PreparedMaterial2d>>,
    render_mesh_instances: ResMut<'w, RenderMesh2dInstances>,
    render_material_instances: Res<'w, RenderMaterial2dInstances>,
    transparent_render_phases: Res<'w, ViewSortedRenderPhases<Transparent2d>>,
    opaque_render_phases: Res<'w, ViewBinnedRenderPhases<Opaque2d>>,
    alpha_mask_render_phases: Res<'w, ViewBinnedRenderPhases<AlphaMask2d>>,
    views: Query<
        'w,
        's,
        (
            &'static MainEntity,
            &'static ExtractedView,
            &'static RenderVisibleEntities,
        ),
    >,
    view_key_cache: Res<'w, ViewKeyCache>,
    specialized_material2d_pipeline_cache: ResMut<'w, SpecializedMaterial2dPipelineCache>,
    dirty_specializations: Res<'w, DirtySpecializations>,
    pending_mesh_material2d_queues: ResMut<'w, PendingMeshMaterial2dQueues>,
}

/// Specializes and compiles pipelines for 2D materials that haven't been seen
/// yet.
pub fn specialize_material2d_meshes(
    world: &mut World,
    state: &mut SystemState<SpecializeMaterial2dMeshesSystemParam>,
    mut work_items: Local<Vec<Specialization2dWorkItem>>,
    mut all_views: Local<HashSet<RetainedViewEntity, FixedHasher>>,
) {
    work_items.clear();
    all_views.clear();

    {
        let SpecializeMaterial2dMeshesSystemParam {
            render_meshes,
            render_materials,
            mut render_mesh_instances,
            render_material_instances,
            transparent_render_phases,
            opaque_render_phases,
            alpha_mask_render_phases,
            views,
            view_key_cache,
            mut specialized_material2d_pipeline_cache,
            dirty_specializations,
            mut pending_mesh_material2d_queues,
        } = state.get_mut(world).unwrap();

        if render_material_instances.is_empty() {
            return;
        }

        for (view_entity, view, visible_entities) in &views {
            all_views.insert(view.retained_view_entity);

            if !transparent_render_phases.contains_key(&view.retained_view_entity)
                && !opaque_render_phases.contains_key(&view.retained_view_entity)
                && !alpha_mask_render_phases.contains_key(&view.retained_view_entity)
            {
                continue;
            }

            let Some(view_key) = view_key_cache.get(view_entity) else {
                continue;
            };

            let view_specialized_material_pipeline_cache = specialized_material2d_pipeline_cache
                .entry(view.retained_view_entity)
                .or_default();

            let Some(visible_entities) = visible_entities.get::<Mesh2d>() else {
                continue;
            };

            // Remove cached pipeline IDs corresponding to entities that either
            // have been removed or need to be re-specialized.
            if dirty_specializations.must_wipe_specializations_for_view(view.retained_view_entity) {
                view_specialized_material_pipeline_cache.clear();
            } else {
                for &renderable_entity in dirty_specializations.iter_to_despecialize() {
                    view_specialized_material_pipeline_cache.remove(&renderable_entity);
                }
            }

            let Some(view_pending_mesh_material2d_queues) =
                pending_mesh_material2d_queues.get_mut(&view.retained_view_entity)
            else {
                continue;
            };

            // Now process all 2D meshes that need to be re-specialized.
            for (render_entity, visible_entity) in dirty_specializations.iter_to_specialize(
                view.retained_view_entity,
                visible_entities,
                &view_pending_mesh_material2d_queues.prev_frame,
            ) {
                if view_specialized_material_pipeline_cache.contains_key(visible_entity) {
                    continue;
                }

                let Some(material_asset_id) = render_material_instances.get(visible_entity) else {
                    // Entity doesn't have this material type. Skip it; the
                    // correct material type's specialize system will handle it.
                    continue;
                };
                let Some(mesh_instance) = render_mesh_instances.get_mut(visible_entity) else {
                    // We couldn't fetch the mesh instance, probably because the
                    // material hasn't been loaded yet. Add the entity to the
                    // list of pending mesh materials and bail.
                    view_pending_mesh_material2d_queues
                        .current_frame
                        .insert((*render_entity, *visible_entity));
                    continue;
                };
                let Some(material_2d) = render_materials.get(*material_asset_id) else {
                    // We couldn't fetch the material instance, probably because the
                    // material hasn't been loaded yet. Add the entity to the list
                    // of pending mesh materials and bail.
                    view_pending_mesh_material2d_queues
                        .current_frame
                        .insert((*render_entity, *visible_entity));
                    continue;
                };
                let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                    continue;
                };

                let mut mesh_pipeline_key_bits: Mesh2dPipelineKey =
                    material_2d.properties.mesh_pipeline_key_bits.downcast();
                mesh_pipeline_key_bits.insert(alpha_mode_pipeline_key_2d(
                    material_2d.properties.alpha_mode,
                ));
                let mesh_key = *view_key
                    | Mesh2dPipelineKey::from_bits_retain(mesh.key_bits.bits())
                    | mesh_pipeline_key_bits;

                work_items.push(Specialization2dWorkItem {
                    visible_entity: *visible_entity,
                    retained_view_entity: view.retained_view_entity,
                    mesh_key,
                    layout: mesh.layout.clone(),
                    properties: material_2d.properties.clone(),
                    material_type_id: material_asset_id.type_id(),
                });
            }
        }
    }

    for item in work_items.drain(..) {
        let key = ErasedMaterialPipelineKey {
            type_id: item.material_type_id,
            mesh_key: ErasedMeshPipelineKey::new(item.mesh_key),
            material_key: item.properties.material_key.clone(),
        };

        let pipeline_id = base_specialize(world, key, &item.layout, &item.properties);

        let pipeline_id = match pipeline_id {
            Ok(id) => id,
            Err(err) => {
                error!("{}", err);
                continue;
            }
        };

        let mut specialized_material_pipeline_cache =
            world.resource_mut::<SpecializedMaterial2dPipelineCache>();
        let view_specialized_material_pipeline_cache = specialized_material_pipeline_cache
            .entry(item.retained_view_entity)
            .or_default();
        view_specialized_material_pipeline_cache.insert(item.visible_entity, pipeline_id);
    }

    world
        .resource_mut::<SpecializedMaterial2dPipelineCache>()
        .retain(|view, _| all_views.contains(view));
}

/// An internal type that [`specialize_material2d_meshes`] uses to store
/// specialization jobs.
pub struct Specialization2dWorkItem {
    visible_entity: MainEntity,
    retained_view_entity: RetainedViewEntity,
    mesh_key: Mesh2dPipelineKey,
    layout: MeshVertexBufferLayoutRef,
    properties: Arc<MaterialProperties>,
    material_type_id: TypeId,
}

/// Iterates over all 2D mesh instances that have changed, adding and removing
/// them from render batches and bins as appropriate.
pub fn queue_material2d_meshes(
    (render_meshes, render_materials): (
        Res<RenderAssets<RenderMesh>>,
        Res<ErasedRenderAssets<PreparedMaterial2d>>,
    ),
    mut render_mesh_instances: ResMut<RenderMesh2dInstances>,
    render_material_instances: Res<RenderMaterial2dInstances>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque2d>>,
    mut alpha_mask_render_phases: ResMut<ViewBinnedRenderPhases<AlphaMask2d>>,
    views: Query<(&ExtractedView, &RenderVisibleEntities)>,
    dirty_specializations: Res<DirtySpecializations>,
    mut pending_mesh_material2d_queues: ResMut<PendingMeshMaterial2dQueues>,
    specialized_material_pipeline_cache: ResMut<SpecializedMaterial2dPipelineCache>,
    mut mesh_instances_queued_this_iteration_scratch_space: Local<MainEntityHashSet>,
) {
    if render_material_instances.is_empty() {
        return;
    }

    for (view, visible_entities) in &views {
        let Some(view_specialized_material_pipeline_cache) =
            specialized_material_pipeline_cache.get(&view.retained_view_entity)
        else {
            continue;
        };

        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let Some(alpha_mask_phase) = alpha_mask_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let Some(visible_entities) = visible_entities.get::<Mesh2d>() else {
            continue;
        };

        let view_pending_mesh_material2d_queues = pending_mesh_material2d_queues
            .get_mut(&view.retained_view_entity)
            .expect(
                "View pending mesh material 2D queues should have been created in \
                 `prepare_pending_mesh_material2d_queues`",
            );

        // Remove entities that became invisible or fully lost their
        // mesh/material from the render phases. Entities that are also
        // in `changed_renderables` are switching material type and will
        // be handled by the inline dequeue in the queue loop below.
        for main_entity in visible_entities
            .removed_entities
            .iter()
            .map(|(_, main_entity)| main_entity)
            .chain(
                dirty_specializations
                    .removed_renderables
                    .iter()
                    .filter(|e| !dirty_specializations.changed_renderables.contains(*e)),
            )
        {
            transparent_phase.remove(Entity::PLACEHOLDER, *main_entity);
            opaque_phase.remove(*main_entity);
            alpha_mask_phase.remove(*main_entity);
        }

        // Now iterate over all newly-visible entities and those that need
        // specialization.
        for (render_entity, visible_entity) in dirty_specializations.iter_to_queue(
            view.retained_view_entity,
            visible_entities,
            &view_pending_mesh_material2d_queues.prev_frame,
            &mut mesh_instances_queued_this_iteration_scratch_space,
        ) {
            let Some(pipeline_id) = view_specialized_material_pipeline_cache
                .get(visible_entity)
                .copied()
            else {
                continue;
            };

            let Some(material_asset_id) = render_material_instances.get(visible_entity) else {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.get_mut(visible_entity) else {
                continue;
            };
            let Some(material_2d) = render_materials.get(*material_asset_id) else {
                // We couldn't fetch the material instance, probably because the
                // material hasn't been loaded yet. Add the entity to the list
                // of pending mesh materials and bail.
                view_pending_mesh_material2d_queues
                    .current_frame
                    .insert((*render_entity, *visible_entity));
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            // Remove old phase item before re-adding. This handles bin
            // key changes and is safe even if the entity wasn't previously
            // queued. Doing this after the pipeline check ensures
            // each material type only dequeues its own entities.
            transparent_phase.remove(Entity::PLACEHOLDER, *visible_entity);
            opaque_phase.remove(*visible_entity);
            alpha_mask_phase.remove(*visible_entity);

            let mesh_z = mesh_instance.transforms.world_from_local.translation.z;

            // We don't support multidraw yet for 2D meshes, so we use this
            // custom logic to generate the `BinnedRenderPhaseType` instead of
            // `BinnedRenderPhaseType::mesh`, which can return
            // `BinnedRenderPhaseType::MultidrawableMesh` if the hardware
            // supports multidraw.
            let binned_render_phase_type = if mesh_instance.automatic_batching {
                BinnedRenderPhaseType::BatchableMesh
            } else {
                BinnedRenderPhaseType::UnbatchableMesh
            };

            match material_2d.properties.alpha_mode {
                AlphaMode::Opaque => {
                    let Some(draw_function) = material_2d
                        .properties
                        .get_draw_function(Pass2dOpaqueDrawFunction)
                    else {
                        continue;
                    };
                    let bin_key = Opaque2dBinKey {
                        pipeline: pipeline_id,
                        draw_function,
                        asset_id: mesh_instance.mesh_asset_id.into(),
                        material_bind_group_index: Some(material_2d.binding.group.0),
                    };
                    opaque_phase.add(
                        BatchSetKey2d {
                            indexed: mesh.indexed(),
                        },
                        bin_key,
                        (*render_entity, *visible_entity),
                        InputUniformIndex::default(),
                        binned_render_phase_type,
                    );
                }
                AlphaMode::Mask(_) => {
                    let Some(draw_function) = material_2d
                        .properties
                        .get_draw_function(Pass2dAlphaMaskDrawFunction)
                    else {
                        continue;
                    };
                    let bin_key = AlphaMask2dBinKey {
                        pipeline: pipeline_id,
                        draw_function,
                        asset_id: mesh_instance.mesh_asset_id.into(),
                        material_bind_group_index: Some(material_2d.binding.group.0),
                    };
                    alpha_mask_phase.add(
                        BatchSetKey2d {
                            indexed: mesh.indexed(),
                        },
                        bin_key,
                        (*render_entity, *visible_entity),
                        InputUniformIndex::default(),
                        binned_render_phase_type,
                    );
                }
                _ => {
                    let Some(draw_function) = material_2d
                        .properties
                        .get_draw_function(Pass2dTransparentDrawFunction)
                    else {
                        continue;
                    };
                    // We have to use `Entity::PLACEHOLDER` as the render entity
                    // so that we can dequeue the items later with
                    // `iter_to_dequeue` above.
                    // Items can be removed from binned phases by knowing their
                    // main entity alone, but items can only be removed from
                    // sorted phases if both the render entity and main world
                    // entity are known. So we have to use a fixed value,
                    // `Entity::PLACEHOLDER`, here, because
                    // `DirtySpecializations` only tracks main world entities,
                    // not render world ones.
                    // Really, in the future we should get rid of the render
                    // entity field here entirely, but we currently can't do so
                    // because UI creates multiple render entities for each main
                    // entity in its sorted phases.
                    transparent_phase.add_retained(Transparent2d {
                        entity: (Entity::PLACEHOLDER, *visible_entity),
                        draw_function,
                        pipeline: pipeline_id,
                        // NOTE: Back-to-front ordering for transparent with ascending sort means far should have the
                        // lowest sort key and getting closer should increase. As we have
                        // -z in front of the camera, the largest distance is -far with values increasing toward the
                        // camera. As such we can just use mesh_z as the distance
                        sort_key: FloatOrd(mesh_z + material_2d.properties.depth_bias),
                        // Batching is done in batch_and_prepare_render_phase
                        batch_range: 0..1,
                        extra_index: PhaseItemExtraIndex::None,
                        extracted_index: usize::MAX,
                        indexed: mesh.indexed(),
                    });
                }
            }
        }
    }
}

/// The bind group ID that a single 2D material has been assigned to.
#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut)]
pub struct Material2dBindGroupId(pub Option<BindGroupId>);

/// Data prepared for a [`Material2d`] instance.
pub struct PreparedMaterial2d {
    /// Where the material is stored in the bind group allocator.
    pub binding: MaterialBindingId,
    /// Common properties for the material.
    pub properties: Arc<MaterialProperties>,
}

impl<M: Material2d> ErasedRenderAsset for MeshMaterial2d<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type SourceAsset = M;
    type ErasedAsset = PreparedMaterial2d;

    type Param = (
        SRes<RenderDevice>,
        SRes<PipelineCache>,
        SResMut<MaterialBindGroupAllocators>,
        SResMut<RenderMaterialBindings>,
        SRes<DrawFunctions<Opaque2d>>,
        SRes<DrawFunctions<AlphaMask2d>>,
        SRes<DrawFunctions<Transparent2d>>,
        SRes<Material2dShaders<M>>,
        M::Param,
    );

    fn prepare_asset(
        material: Self::SourceAsset,
        material_id: AssetId<Self::SourceAsset>,
        (
            render_device,
            pipeline_cache,
            bind_group_allocators,
            render_material_bindings,
            opaque_draw_functions,
            alpha_mask_draw_functions,
            transparent_draw_functions,
            material_shaders,
            material_param,
        ): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::ErasedAsset, PrepareAssetError<Self::SourceAsset>> {
        let material_layout = M::bind_group_layout_descriptor(render_device);

        let material_binding_id = render_material_bindings.prepare_material(
            &material,
            material_id,
            material_param,
            &material_layout,
            bind_group_allocators,
            render_device,
            pipeline_cache,
        )?;

        let mut mesh_pipeline_key_bits = Mesh2dPipelineKey::empty();
        mesh_pipeline_key_bits.insert(alpha_mode_pipeline_key_2d(material.alpha_mode().into()));
        let mesh_pipeline_key_bits = ErasedMeshPipelineKey::new(mesh_pipeline_key_bits);

        let render_phase_type = match material.alpha_mode() {
            AlphaMode2d::Opaque => RenderPhaseType::Opaque,
            AlphaMode2d::Mask(_) => RenderPhaseType::AlphaMask,
            AlphaMode2d::Blend => RenderPhaseType::Transparent,
        };

        let draw_opaque_2d = opaque_draw_functions.read().id::<DrawMaterial2d>();
        let draw_alpha_mask_2d = alpha_mask_draw_functions.read().id::<DrawMaterial2d>();
        let draw_transparent_2d = transparent_draw_functions.read().id::<DrawMaterial2d>();

        let draw_functions = SmallVec::from_iter([
            (Pass2dOpaqueDrawFunction.intern(), draw_opaque_2d),
            (Pass2dAlphaMaskDrawFunction.intern(), draw_alpha_mask_2d),
            (Pass2dTransparentDrawFunction.intern(), draw_transparent_2d),
        ]);

        let shaders = material_shaders.shaders.clone();

        let bindless = material_uses_bindless_resources::<M>(render_device);
        let bind_group_data = material.bind_group_data();
        let material_key = ErasedMaterialKey::new(bind_group_data);

        Ok(PreparedMaterial2d {
            binding: material_binding_id,
            properties: Arc::new(MaterialProperties {
                depth_bias: material.depth_bias(),
                alpha_mode: material.alpha_mode().into(),
                material_layout: Some(material_layout),
                bindless,
                base_specialize: Some(base_specialize),
                user_specialize: Some(user_specialize::<M>),
                mesh_pipeline_key_bits,
                render_method: OpaqueRendererMethod::Forward,
                material_key,
                render_phase_type,
                reads_view_transmission_texture: false,
                draw_functions,
                shaders,
                prepass_specialize: None,
                shadows_enabled: false,
                prepass_enabled: false,
                oit_enabled: false,
            }),
        })
    }
}

/// Creates a [`Material2dPipelineSpecializer`] and uses it to specialize a
/// single 2D material.
pub fn base_specialize(
    world: &mut World,
    key: ErasedMaterialPipelineKey,
    layout: &MeshVertexBufferLayoutRef,
    properties: &Arc<MaterialProperties>,
) -> Result<CachedRenderPipelineId, SpecializedMeshPipelineError> {
    world.resource_scope(
        |world, mut pipelines: Mut<SpecializedMeshPipelines<Material2dPipelineSpecializer>>| {
            let mesh2d_pipeline = world.resource::<Mesh2dPipeline>().clone();
            let pipeline_cache = world.resource::<PipelineCache>();

            let specializer = Material2dPipelineSpecializer {
                pipeline: Material2dPipeline { mesh2d_pipeline },
                properties: properties.clone(),
            };

            pipelines.specialize(pipeline_cache, &specializer, key, layout)
        },
    )
}

/// Calls the [`Material2d::specialize`] function for a single material in order
/// to do any custom specializations that the material wishes.
fn user_specialize<M>(
    pipeline: &dyn Any,
    descriptor: &mut RenderPipelineDescriptor,
    mesh_layout: &MeshVertexBufferLayoutRef,
    erased_key: ErasedMaterialPipelineKey,
) -> Result<(), SpecializedMeshPipelineError>
where
    M: Material2d,
    M::Data: Hash + Clone,
{
    let pipeline = pipeline.downcast_ref::<Material2dPipeline>().unwrap();
    let material_key = erased_key.material_key.to_key();
    let mesh_key: Mesh2dPipelineKey = erased_key.mesh_key.downcast();
    M::specialize(
        pipeline,
        descriptor,
        mesh_layout,
        Material2dKey {
            mesh_key,
            bind_group_data: material_key,
        },
    )
}

/// Identifies the vertex shader for a 2D material.
#[derive(ShaderLabel, Debug, Hash, PartialEq, Eq, Clone, Default)]
pub struct Material2dVertexShader;
/// Identifies the fragment shader for a 2D material.
#[derive(ShaderLabel, Debug, Hash, PartialEq, Eq, Clone, Default)]
pub struct Material2dFragmentShader;

/// The draw function that draws binned opaque 2D meshes.
#[derive(DrawFunctionLabel, Debug, Hash, PartialEq, Eq, Clone, Default)]
pub struct Pass2dOpaqueDrawFunction;
/// The draw function that draws binned alpha masked 2D meshes.
#[derive(DrawFunctionLabel, Debug, Hash, PartialEq, Eq, Clone, Default)]
pub struct Pass2dAlphaMaskDrawFunction;
/// The draw function that draws sorted 2D meshes with alpha-blended
/// transparency.
#[derive(DrawFunctionLabel, Debug, Hash, PartialEq, Eq, Clone, Default)]
pub struct Pass2dTransparentDrawFunction;

/// A resource that caches resolved shader handles for a specific material type.
#[derive(Resource)]
pub struct Material2dShaders<M>
where
    M: Material2d,
{
    shaders: SmallVec<[(InternedShaderLabel, Handle<Shader>); 6]>,
    marker: PhantomData<M>,
}

impl<M> Default for Material2dShaders<M>
where
    M: Material2d,
{
    fn default() -> Self {
        Material2dShaders {
            shaders: SmallVec::new(),
            marker: PhantomData,
        }
    }
}

impl<M> Material2dShaders<M>
where
    M: Material2d,
{
    /// Creates a [`Material2dShaders`] from the given vertex and fragment
    /// shaders.
    pub fn with_shader_cache(
        shaders: SmallVec<[(InternedShaderLabel, Handle<Shader>); 6]>,
    ) -> Material2dShaders<M> {
        Material2dShaders {
            shaders,
            marker: PhantomData,
        }
    }
}

/// Initializes the vertex and fragment shaders for a single 2D material.
fn initialize_material2d_shaders<M>(
    render_world: &World,
) -> SmallVec<[(InternedShaderLabel, Handle<Shader>); 6]>
where
    M: Material2d,
{
    let asset_server = render_world.resource::<AssetServer>();
    let mut shaders = SmallVec::new();

    let mut add_shader = |label: InternedShaderLabel, shader_ref: ShaderRef| {
        let maybe_shader = match shader_ref {
            ShaderRef::Default => None,
            ShaderRef::Handle(handle) => Some(handle),
            ShaderRef::Path(path) => Some(asset_server.load(path)),
        };
        if let Some(shader) = maybe_shader {
            shaders.push((label, shader));
        }
    };

    add_shader(Material2dVertexShader.intern(), M::vertex_shader());
    add_shader(Material2dFragmentShader.intern(), M::fragment_shader());

    shaders
}
