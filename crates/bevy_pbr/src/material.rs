use alloc::sync::Arc;

use crate::*;
use bevy_asset::prelude::AssetChanged;
use bevy_asset::{Asset, AssetEventSystems, AssetId, AssetServer, UntypedAssetId};
use bevy_camera::visibility::ViewVisibility;
use bevy_camera::ScreenSpaceTransmissionQuality;
use bevy_core_pipeline::tonemapping::Tonemapping;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::change_detection::Tick;
use bevy_ecs::system::{ReadOnlySystemParam, SystemChangeTick};
use bevy_ecs::{
    prelude::*,
    system::{
        lifetimeless::{SRes, SResMut},
        SystemParamItem,
    },
};
use bevy_mesh::{
    mark_3d_meshes_as_changed_if_their_assets_changed, BaseMeshPipelineKey, Mesh3d,
    MeshVertexBufferLayoutRef,
};
use bevy_platform::collections::hash_map::Entry;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_platform::hash::{FixedHasher, NoOpHash};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_render::batching::GetFullBatchData;
use bevy_render::camera::extract_cameras;
use bevy_render::erased_render_asset::{
    ErasedRenderAsset, ErasedRenderAssetPlugin, ErasedRenderAssets, PrepareAssetError,
};
use bevy_render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy_render::render_asset::{prepare_assets, RenderAssets};
use bevy_render::renderer::RenderQueue;
use bevy_render::RenderStartup;
use bevy_render::{
    batching::gpu_preprocessing::GpuPreprocessingSupport,
    extract_resource::ExtractResource,
    mesh::RenderMesh,
    prelude::*,
    render_phase::*,
    render_resource::*,
    renderer::RenderDevice,
    sync_world::MainEntity,
    view::{ExtractedView, Msaa, RenderVisibilityRanges, RetainedViewEntity},
    Extract,
};
use bevy_render::{mesh::allocator::MeshAllocator, sync_world::MainEntityHashMap};
use bevy_render::{texture::FallbackImage, view::RenderVisibleEntities};
use bevy_shader::Shader;
use bevy_utils::Parallel;
use core::any::{Any, TypeId};
use core::hash::{BuildHasher, Hasher};
use core::{hash::Hash, marker::PhantomData};
use smallvec::SmallVec;
use tracing::error;

pub const MATERIAL_BIND_GROUP_INDEX: usize = 3;

pub const PASS_MAX_PHASES: usize = 4;

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
/// # use bevy_mesh::{Mesh, Mesh3d};
/// # use bevy_render::render_resource::AsBindGroup;
/// # use bevy_shader::ShaderRef;
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
/// @group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> color: vec4<f32>;
/// @group(#{MATERIAL_BIND_GROUP}) @binding(1) var color_texture: texture_2d<f32>;
/// @group(#{MATERIAL_BIND_GROUP}) @binding(2) var color_sampler: sampler;
/// ```
pub trait Material: Asset + AsBindGroup + Clone + Sized {
    /// Returns this material's shaders for supported passes.
    ///
    /// When the traditional shader method is used, the corresponding pass's shader in the [`PassShaders`] will be ignored.
    /// Currently, only [`MainPass`], [`DeferredPass`] and [`Prepass`] are supported out of the box.
    fn shaders() -> PassShaders {
        let mut pass_shaders = PassShaders::default();
        pass_shaders.insert(Prepass::id(), ShaderSet::default());
        pass_shaders.insert(DeferredPass::id(), ShaderSet::default());
        pass_shaders.insert(MainPass::id(), ShaderSet::default());
        pass_shaders
    }

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
    /// This allows taking color output from the [`bevy_core_pipeline::core_3d::Opaque3d`] pass as an input, (for screen-space transmission) but requires
    /// rendering to take place in a separate [`bevy_core_pipeline::core_3d::Transmissive3d`] pass.
    fn reads_view_transmission_texture(&self) -> bool {
        false
    }

    /// Controls if the prepass is enabled for the Material.
    /// For more information about what a prepass is, see the [`bevy_core_pipeline::prepass`] docs.
    #[inline]
    fn enable_prepass() -> bool {
        true
    }

    /// Controls if shadows are enabled for the Material.
    #[inline]
    fn enable_shadows() -> bool {
        true
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
    fn deferred_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Returns this material's [`crate::meshlet::MeshletMesh`] fragment shader. If [`ShaderRef::Default`] is returned,
    /// the default meshlet mesh fragment shader will be used.
    ///
    /// This is part of an experimental feature, and is unnecessary to implement unless you are using `MeshletMesh`'s.
    ///
    /// See [`crate::meshlet::MeshletMesh`] for limitations.
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
    #[cfg(feature = "meshlet")]
    fn meshlet_mesh_deferred_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }

    /// Customizes the default [`RenderPipelineDescriptor`] for a specific entity using the entity's
    /// [`MaterialPipelineKey`] and [`MeshVertexBufferLayoutRef`] as input.
    #[expect(
        unused_variables,
        reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
    )]
    #[inline]
    fn specialize(
        pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
        // pass_id: PassId,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(())
    }
}

/// A set of shaders for a rasterization pass,
/// containing a vertex shader and fragment shader.
#[derive(Default, Clone)]
pub struct ShaderSet {
    pub vertex: ShaderRef,
    pub fragment: ShaderRef,
}

/// A map for storing the shaders for each pass used by a [`Material`].
pub type PassShaders = HashMap<PassId, ShaderSet, NoOpHash>;

/// A unique identifier for a [`MeshPass`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deref)]
pub struct PassId(TypeId);

impl PassId {
    /// Creates a new [`PassId`] from a [`MeshPass`].
    pub fn of<P: MeshPass>() -> Self {
        PassId(TypeId::of::<P>())
    }
}

/// A trait for defining a render pass that can be used by [`Material`].
///
/// Add the pass to [`bevy_camera::Camera3d`]s to mark views for [`MeshPassPlugin`] rendering.
pub trait MeshPass
where
    Self: ExtractComponent + Default,
    <Self::Specializer as SpecializedMeshPipeline>::Key: Sync + Send,
    <Self::RenderCommand as RenderCommand<Phase1<Self>>>::Param: ReadOnlySystemParam,
    <Self::RenderCommand as RenderCommand<Phase2<Self>>>::Param: ReadOnlySystemParam,
    <Self::RenderCommand as RenderCommand<Phase3<Self>>>::Param: ReadOnlySystemParam,
    <Self::RenderCommand as RenderCommand<Phase4<Self>>>::Param: ReadOnlySystemParam,
{
    /// The pass responsible for checking view specialization.
    ///
    /// You can reuse existing pass implementations. If you want to create your own,
    /// this would be `Self`. Check out [`MainPass`] for more details.
    type ViewKeySource: MeshPass;

    /// The specializer for creating [`RenderPipelineDescriptor`].
    type Specializer: PipelineSpecializer;

    /// The [`PhaseItem`]s processed by this pass.
    ///
    /// Each `PhaseItem` must implement [`PhaseItemExt`].
    /// Currently, the maximum number of `PhaseItem`s in a pass is 4.
    ///
    /// If the desired effect does not rely on a specific pass order, such as Hull Outline,
    /// you can reuse existing `PhaseItem`s, as they will ultimately be processed by the same
    /// render graph node. If your effect depends on a specific pass order, such as a custom
    /// prepass, then you will need to create a newtype or completely rebuild your own `PhaseItem`,
    /// and implement the corresponding render graph node along with other necessary systems.
    ///
    /// ## Example
    /// ```ignore
    /// type PhaseItems = (Opaque3d, AlphaMask3d, Transmissive3d, Transparent3d);
    /// ```
    type PhaseItems: PhaseItems<Self>;

    // NOTE AFTER #21021: Maybe we can move this to `PhaseItemExt`.
    //
    // NOTE BEFORE #21021: Theoretically, the relationship between PhaseItem and RenderCommand
    // is many-to-many, but I haven't seen many use cases of this. For simplicity
    // of implementation, I’m keeping it this way for now.
    //
    /// The `RenderCommand`s used for rendering [`PhaseItem`]s.
    /// See [`RenderCommand`] for more details.
    type RenderCommand: Send
        + Sync
        + RenderCommand<Phase1<Self>>
        + RenderCommand<Phase2<Self>>
        + RenderCommand<Phase3<Self>>
        + RenderCommand<Phase4<Self>>;

    /// The identifier for this pass.
    fn id() -> PassId {
        PassId::of::<Self>()
    }
}

/// A trait for converting a tuple of [`PhaseItem`]s into associated types.
pub trait PhaseItems<P> {
    type Phase1: PhaseItemExt;
    type Phase2: PhaseItemExt;
    type Phase3: PhaseItemExt;
    type Phase4: PhaseItemExt;

    fn count() -> usize;
}

// Fake singleton for [`MeshPassPlugin`]
#[derive(Resource, Default)]
struct MeshPassPluginLoaded;

/// A plugin for adding a [`MeshPass`] that can be used by [`Material`].
///
/// Currently handles the specialization and queuing stages.
#[derive(Default)]
pub struct MeshPassPlugin<P> {
    pub debug_flags: RenderDebugFlags,
    _marker: PhantomData<P>,
}

impl<P: MeshPass> MeshPassPlugin<P> {
    pub fn new(debug_flags: RenderDebugFlags) -> Self {
        Self {
            debug_flags,
            _marker: PhantomData,
        }
    }
}

impl<P: MeshPass> Plugin for MeshPassPlugin<P> {
    fn build(&self, app: &mut App) {
        add_pass_phase_plugins::<P>(app, self.debug_flags);
        app.add_plugins(ExtractComponentPlugin::<P>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // For all instances of PassPlugin
        render_app
            .init_resource::<EntitySpecializationTicks>()
            .init_resource::<SpecializedMaterialPipelineCache<P>>()
            .init_resource::<MeshPassSpecializedMeshPipelines<P, P::Specializer>>()
            .init_resource::<ViewKeyCache<P>>() // Double check
            .init_resource::<ViewSpecializationTicks<P>>() // Double check
            .init_resource::<RenderMaterialInstances>()
            .init_resource::<MaterialBindGroupAllocators>()
            .init_resource::<EntitiesNeedingSweep>()
            .add_systems(
                ExtractSchedule,
                late_sweep_entities_needing_specialization::<P>
                    .after(MaterialEarlySweepEntitiesNeedingSpecializationSystems)
                    .before(late_sweep_material_instances),
            )
            .add_systems(
                Render,
                (
                    specialize_material_meshes::<P>
                        .in_set(RenderSystems::PrepareMeshes)
                        .after(prepare_assets::<RenderMesh>)
                        .after(collect_meshes_for_gpu_building)
                        .after(set_mesh_motion_vector_flags),
                    queue_material_meshes::<P>.in_set(RenderSystems::QueueMeshes),
                ),
            );

        // Fake singleton start
        if render_app
            .world()
            .contains_resource::<MeshPassPluginLoaded>()
        {
            return;
        }
        render_app
            .init_resource::<MeshPassPluginLoaded>()
            .add_systems(
                Render,
                (
                    prepare_material_bind_groups,
                    write_material_bind_group_buffers,
                )
                    .chain()
                    .in_set(RenderSystems::PrepareBindGroups),
            );
    }
}

pub fn add_pass_phase_plugins<P: MeshPass>(app: &mut App, debug_flags: RenderDebugFlags) {
    let valid_phase_count = P::PhaseItems::count();

    if valid_phase_count > 0 {
        app.add_plugins(MeshPassPhasePlugin::<P, Phase1<P>>::new(0, debug_flags));
    }
    if valid_phase_count > 1 {
        app.add_plugins(MeshPassPhasePlugin::<P, Phase2<P>>::new(1, debug_flags));
    }
    if valid_phase_count > 2 {
        app.add_plugins(MeshPassPhasePlugin::<P, Phase3<P>>::new(2, debug_flags));
    }
    if valid_phase_count > 3 {
        app.add_plugins(MeshPassPhasePlugin::<P, Phase4<P>>::new(3, debug_flags));
    }
}

pub struct MeshPassPhasePlugin<P, PIE> {
    phase_index: usize,
    debug_flags: RenderDebugFlags,
    _marker: PhantomData<(P, PIE)>,
}

impl<P, PIE> MeshPassPhasePlugin<P, PIE> {
    pub fn new(phase_index: usize, debug_flags: RenderDebugFlags) -> Self {
        Self {
            phase_index,
            debug_flags,
            _marker: PhantomData,
        }
    }
}

impl<P, PIE> Plugin for MeshPassPhasePlugin<P, PIE>
where
    P: MeshPass,
    PIE: PhaseItemExt,
    P::RenderCommand: RenderCommand<PIE>,
    <P::RenderCommand as RenderCommand<PIE>>::Param: ReadOnlySystemParam,
{
    fn build(&self, app: &mut App) {
        // NOTE: The resource `PIE::RenderPhases` is indirectly initialized `by PIE::PhasePlugin`.
        app.add_plugins(PIE::PhasePlugin::new(self.debug_flags));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<PassPhaseDrawFunctions>()
            .init_resource::<DrawFunctions<PIE>>()
            .add_render_command::<PIE, P::RenderCommand>()
            .add_systems(
                RenderStartup,
                insert_pass_phase_draw_functions::<P, PIE>.with_input(self.phase_index),
            );
    }
}

#[derive(Resource, Default)]
struct MaterialPluginLoaded;

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given [`Material`]
/// asset type.
pub struct MaterialPlugin<M: Material> {
    /// Debugging flags that can optionally be set when constructing the renderer.
    pub debug_flags: RenderDebugFlags,
    pub _marker: PhantomData<M>,
}

impl<M: Material> Default for MaterialPlugin<M> {
    fn default() -> Self {
        Self {
            debug_flags: RenderDebugFlags::default(),
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
            .register_type::<MeshMaterial3d<M>>()
            .init_resource::<EntitiesNeedingSpecialization<M>>()
            .init_resource::<EntitiesNeedingSweep>()
            .add_plugins((ErasedRenderAssetPlugin::<MeshMaterial3d<M>>::default(),))
            .add_systems(
                PostUpdate,
                (
                    mark_meshes_as_changed_if_their_materials_changed::<M>.ambiguous_with_all(),
                    check_entities_needing_specialization::<M>.after(AssetEventSystems),
                )
                    .after(mark_3d_meshes_as_changed_if_their_assets_changed),
            );

        if M::enable_shadows() {
            app.add_systems(
                PostUpdate,
                check_light_entities_needing_specialization::<M>
                    .after(check_entities_needing_specialization::<M>),
            );
        }

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(RenderStartup, add_material_bind_group_allocator::<M>)
            .add_systems(
                ExtractSchedule,
                (
                    extract_mesh_materials::<M>.in_set(MaterialExtractionSystems),
                    early_sweep_material_instances::<M>
                        .after(MaterialExtractionSystems)
                        .before(late_sweep_material_instances),
                    // See the comments in
                    // `early_sweep_entities_needing_specialization` for an
                    // explanation of why the systems are ordered this way.
                    extract_entities_needs_specialization::<M>
                        .in_set(MaterialExtractEntitiesNeedingSpecializationSystems),
                    early_sweep_entities_needing_specialization::<M>
                        .in_set(MaterialEarlySweepEntitiesNeedingSpecializationSystems)
                        .after(MaterialExtractEntitiesNeedingSpecializationSystems)
                        .after(MaterialExtractionSystems)
                        .after(extract_cameras)
                        .before(late_sweep_material_instances),
                ),
            );

        if render_app
            .world()
            .contains_resource::<MaterialPluginLoaded>()
        {
            return;
        }
        render_app
            .init_resource::<MaterialPluginLoaded>()
            .add_systems(ExtractSchedule, late_sweep_material_instances);
    }
}

fn add_material_bind_group_allocator<M: Material>(
    render_device: Res<RenderDevice>,
    mut bind_group_allocators: ResMut<MaterialBindGroupAllocators>,
) {
    bind_group_allocators.insert(
        TypeId::of::<M>(),
        MaterialBindGroupAllocator::new(
            &render_device,
            M::label(),
            material_uses_bindless_resources::<M>(&render_device)
                .then(|| M::bindless_descriptor())
                .flatten(),
            M::bind_group_layout_descriptor(&render_device),
            M::bindless_slot_count(),
        ),
    );
}

/// A dummy [`AssetId`] that we use as a placeholder whenever a mesh doesn't
/// have a material.
///
/// See the comments in [`RenderMaterialInstances::mesh_material`] for more
/// information.
pub(crate) static DUMMY_MESH_MATERIAL: AssetId<StandardMaterial> =
    AssetId::<StandardMaterial>::invalid();

/// A key uniquely identifying a specialized [`MaterialPipeline`].
pub struct MaterialPipelineKey<M: Material> {
    pub mesh_key: MeshPipelineKey,
    pub bind_group_data: M::Data,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErasedMaterialPipelineKey {
    pub mesh_key: MeshPipelineKey,
    pub material_key: ErasedMaterialKey,
    pub type_id: TypeId,
}

/// Render pipeline data for a given [`Material`].
#[derive(Resource, Clone)]
pub struct MaterialPipeline {
    pub mesh_pipeline: MeshPipeline,
}

/// Inserts `PhaseItem`'s `DrawFunction`s into [`PassPhaseDrawFunctions`] by their index in [`PhaseItems`].
///
/// This should be called per `PhaseItem`, and the corresponding `RenderCommand` should be registered before calling this.
pub fn insert_pass_phase_draw_functions<P: MeshPass, PIE: PhaseItemExt>(
    InMut(phase_index): InMut<usize>,
    mut pass_phase_draw_functions: ResMut<PassPhaseDrawFunctions>,
    draw_functions: Res<DrawFunctions<PIE>>,
) {
    let draw_function_id = draw_functions
        .read()
        .get_id::<P::RenderCommand>()
        .expect("DrawFunctionId not found for the pass's RenderCommand. Call `add_draw_function` to register it first.");

    if let Some(draw) = pass_phase_draw_functions
        .entry(P::id())
        .or_default()
        .get_mut(*phase_index)
    {
        *draw = Some(draw_function_id);
    }
}

/// A [`RenderCommand`] for [`MainPass`].
pub type DrawMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMeshBindGroup<2>,
    SetMaterialBindGroup<MATERIAL_BIND_GROUP_INDEX>,
    DrawMesh,
);

/// Sets the bind group for a given [`Material`] at the configured `I` index.
pub struct SetMaterialBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMaterialBindGroup<I> {
    type Param = (
        SRes<ErasedRenderAssets<PreparedMaterial>>,
        SRes<RenderMaterialInstances>,
        SRes<MaterialBindGroupAllocators>,
    );
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: Option<()>,
        (materials, material_instances, material_bind_group_allocator): SystemParamItem<
            'w,
            '_,
            Self::Param,
        >,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let materials = materials.into_inner();
        let material_instances = material_instances.into_inner();
        let material_bind_group_allocators = material_bind_group_allocator.into_inner();

        let Some(material_instance) = material_instances.instances.get(&item.main_entity()) else {
            return RenderCommandResult::Skip;
        };
        let Some(material_bind_group_allocator) =
            material_bind_group_allocators.get(&material_instance.asset_id.type_id())
        else {
            return RenderCommandResult::Skip;
        };
        let Some(material) = materials.get(material_instance.asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(material_bind_group) = material_bind_group_allocator.get(material.binding.group)
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

/// Stores all extracted instances of all [`Material`]s in the render world.
#[derive(Resource, Default)]
pub struct RenderMaterialInstances {
    /// Maps from each entity in the main world to the
    /// [`RenderMaterialInstance`] associated with it.
    pub instances: MainEntityHashMap<RenderMaterialInstance>,
    /// A monotonically-increasing counter, which we use to sweep
    /// [`RenderMaterialInstances::instances`] when the entities and/or required
    /// components are removed.
    pub current_change_tick: Tick,
}

impl RenderMaterialInstances {
    /// Returns the mesh material ID for the entity with the given mesh, or a
    /// dummy mesh material ID if the mesh has no material ID.
    ///
    /// Meshes almost always have materials, but in very specific circumstances
    /// involving custom pipelines they won't. (See the
    /// `specialized_mesh_pipelines` example.)
    pub(crate) fn mesh_material(&self, entity: MainEntity) -> UntypedAssetId {
        match self.instances.get(&entity) {
            Some(render_instance) => render_instance.asset_id,
            None => DUMMY_MESH_MATERIAL.into(),
        }
    }
}

/// The material associated with a single mesh instance in the main world.
///
/// Note that this uses an [`UntypedAssetId`] and isn't generic over the
/// material type, for simplicity.
pub struct RenderMaterialInstance {
    /// The material asset.
    pub asset_id: UntypedAssetId,
    /// The [`RenderMaterialInstances::current_change_tick`] at which this
    /// material instance was last modified.
    pub last_change_tick: Tick,
}

/// A [`SystemSet`] that contains all `extract_mesh_materials` systems.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct MaterialExtractionSystems;

/// A [`SystemSet`] that contains all `extract_entities_needs_specialization`
/// systems.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct MaterialExtractEntitiesNeedingSpecializationSystems;

// NOTE: This is for configuring the order between
// `early_sweep_entities_needs_specialization` and
// `late_sweep_entities_needs_specialization`, which
// have different generic types.
//
/// A [`SystemSet`] that contains all `early_sweep_entities_needs_specialization`
/// systems.
#[derive(SystemSet, Clone, PartialEq, Eq, Debug, Hash)]
pub struct MaterialEarlySweepEntitiesNeedingSpecializationSystems;

/// Deprecated alias for [`MaterialExtractionSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `MaterialExtractionSystems`.")]
pub type ExtractMaterialsSet = MaterialExtractionSystems;

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

pub const fn alpha_mode_render_phase_type(
    alpha_mode: AlphaMode,
    reads_view_transmission_texture: bool,
) -> RenderPhaseType {
    match alpha_mode {
        AlphaMode::Blend | AlphaMode::Premultiplied | AlphaMode::Add | AlphaMode::Multiply => {
            RenderPhaseType::Transparent
        }
        _ if reads_view_transmission_texture => RenderPhaseType::Transmissive,
        AlphaMode::Opaque | AlphaMode::AlphaToCoverage => RenderPhaseType::Opaque,
        AlphaMode::Mask(_) => RenderPhaseType::AlphaMask,
    }
}

/// A system that ensures that
/// [`crate::render::mesh::extract_meshes_for_gpu_building`] re-extracts meshes
/// whose materials changed.
///
/// As [`crate::render::mesh::collect_meshes_for_gpu_building`] only considers
/// meshes that were newly extracted, and it writes information from the
/// [`RenderMaterialInstances`] into the
/// [`crate::render::mesh::MeshInputUniform`], we must tell
/// [`crate::render::mesh::extract_meshes_for_gpu_building`] to re-extract a
/// mesh if its material changed. Otherwise, the material binding information in
/// the [`crate::render::mesh::MeshInputUniform`] might not be updated properly.
/// The easiest way to ensure that
/// [`crate::render::mesh::extract_meshes_for_gpu_building`] re-extracts a mesh
/// is to mark its [`Mesh3d`] as changed, so that's what this system does.
fn mark_meshes_as_changed_if_their_materials_changed<M>(
    mut changed_meshes_query: Query<
        &mut Mesh3d,
        Or<(Changed<MeshMaterial3d<M>>, AssetChanged<MeshMaterial3d<M>>)>,
    >,
) where
    M: Material,
{
    for mut mesh in &mut changed_meshes_query {
        mesh.set_changed();
    }
}

/// Fills the [`RenderMaterialInstances`] resources from the meshes in the
/// scene.
fn extract_mesh_materials<M: Material>(
    mut material_instances: ResMut<RenderMaterialInstances>,
    changed_meshes_query: Extract<
        Query<
            (Entity, &ViewVisibility, &MeshMaterial3d<M>),
            Or<(Changed<ViewVisibility>, Changed<MeshMaterial3d<M>>)>,
        >,
    >,
) {
    let last_change_tick = material_instances.current_change_tick;

    for (entity, view_visibility, material) in &changed_meshes_query {
        if view_visibility.get() {
            material_instances.instances.insert(
                entity.into(),
                RenderMaterialInstance {
                    asset_id: material.id().untyped(),
                    last_change_tick,
                },
            );
        } else {
            material_instances
                .instances
                .remove(&MainEntity::from(entity));
        }
    }
}

/// Removes mesh materials from [`RenderMaterialInstances`] when their
/// [`MeshMaterial3d`] components are removed.
///
/// This is tricky because we have to deal with the case in which a material of
/// type A was removed and replaced with a material of type B in the same frame
/// (which is actually somewhat common of an operation). In this case, even
/// though an entry will be present in `RemovedComponents<MeshMaterial3d<A>>`,
/// we must not remove the entry in `RenderMaterialInstances` which corresponds
/// to material B. To handle this case, we use change ticks to avoid removing
/// the entry if it was updated this frame.
///
/// This is the first of two sweep phases. Because this phase runs once per
/// material type, we need a second phase in order to guarantee that we only
/// bump [`RenderMaterialInstances::current_change_tick`] once.
fn early_sweep_material_instances<M>(
    mut material_instances: ResMut<RenderMaterialInstances>,
    mut removed_materials_query: Extract<RemovedComponents<MeshMaterial3d<M>>>,
) where
    M: Material,
{
    let last_change_tick = material_instances.current_change_tick;

    for entity in removed_materials_query.read() {
        if let Entry::Occupied(occupied_entry) = material_instances.instances.entry(entity.into()) {
            // Only sweep the entry if it wasn't updated this frame.
            if occupied_entry.get().last_change_tick != last_change_tick {
                occupied_entry.remove();
            }
        }
    }
}

/// Removes mesh materials from [`RenderMaterialInstances`] when their
/// [`ViewVisibility`] components are removed.
///
/// This runs after all invocations of `early_sweep_material_instances` and is
/// responsible for bumping [`RenderMaterialInstances::current_change_tick`] in
/// preparation for a new frame.
pub fn late_sweep_material_instances(
    mut material_instances: ResMut<RenderMaterialInstances>,
    mut removed_meshes_query: Extract<RemovedComponents<Mesh3d>>,
) {
    let last_change_tick = material_instances.current_change_tick;

    for entity in removed_meshes_query.read() {
        if let Entry::Occupied(occupied_entry) = material_instances.instances.entry(entity.into()) {
            // Only sweep the entry if it wasn't updated this frame. It's
            // possible that a `ViewVisibility` component was removed and
            // re-added in the same frame.
            if occupied_entry.get().last_change_tick != last_change_tick {
                occupied_entry.remove();
            }
        }
    }

    material_instances
        .current_change_tick
        .set(last_change_tick.get() + 1);
}

pub fn extract_entities_needs_specialization<M>(
    entities_needing_specialization: Extract<Res<EntitiesNeedingSpecialization<M>>>,
    mut entity_specialization_ticks: ResMut<EntitySpecializationTicks>,
    render_material_instances: Res<RenderMaterialInstances>,
    ticks: SystemChangeTick,
) where
    M: Material,
{
    for entity in entities_needing_specialization.iter() {
        // Update the entity's specialization tick with this run's tick
        entity_specialization_ticks.insert(
            (*entity).into(),
            EntitySpecializationTickPair {
                system_tick: ticks.this_run(),
                material_instances_tick: render_material_instances.current_change_tick,
            },
        );
    }
}

/// A system that runs after all instances of
/// [`extract_entities_needs_specialization`] in order to delete specialization
/// ticks for entities that are no longer renderable.
///
/// We delete entities from the [`EntitySpecializationTicks`] table *after*
/// updating it with newly-discovered renderable entities in order to handle the
/// case in which a single entity changes material types. If we naïvely removed
/// entities from that table when their [`MeshMaterial3d<M>`] components were
/// removed, and an entity changed material types, we might end up adding a new
/// set of [`EntitySpecializationTickPair`] for the new material and then
/// deleting it upon detecting the removed component for the old material.
/// Deferring [`early_sweep_entities_needing_specialization`] to the end allows us to
/// detect the case in which another material type updated the entity
/// specialization ticks this frame and avoid deleting it if so.
pub fn early_sweep_entities_needing_specialization<M>(
    mut entity_specialization_ticks: ResMut<EntitySpecializationTicks>,
    mut removed_mesh_material_components: Extract<RemovedComponents<MeshMaterial3d<M>>>,
    mut entities_needing_sweep: ResMut<EntitiesNeedingSweep>,
    render_material_instances: Res<RenderMaterialInstances>,
) where
    M: Material,
{
    // Clean up any despawned entities, we do this first in case the removed material was re-added
    // the same frame, thus will appear both in the removed components list and have been added to
    // the `EntitiesNeedingSpecialization` collection by triggering the `Changed` filter
    //
    // Additionally, we need to make sure that we are careful about materials
    // that could have changed type, e.g. from a `StandardMaterial` to a
    // `CustomMaterial`, as this will also appear in the removed components
    // list. As such, we make sure that this system runs after
    // `extract_entities_needs_specialization` so that the entity specialization
    // tick bookkeeping has already been done, and we can check if the entity's
    // tick was updated this frame.
    for entity in removed_mesh_material_components.read() {
        // If the entity's specialization tick was updated this frame, that
        // means that that entity changed materials this frame. Don't remove the
        // entity from the table in that case.
        if entity_specialization_ticks
            .get(&MainEntity::from(entity))
            .is_some_and(|ticks| {
                ticks.material_instances_tick == render_material_instances.current_change_tick
            })
        {
            continue;
        }

        entity_specialization_ticks.remove(&MainEntity::from(entity));

        // Because `SpecializedMaterialPipelineCache<P>` is per-pass now,
        // defer cleanup to the per-pass system `execute_sweep_entities_needing_specialization`.
        for pass_id in M::shaders().keys() {
            entities_needing_sweep
                .entry(*pass_id)
                .or_default()
                .push(entity);
        }
    }
}

/// Entities needing to be removed from [`SpecializedMaterialPipelineCache`].
#[derive(Resource, Default, Deref, DerefMut)]
pub struct EntitiesNeedingSweep {
    pub entities: HashMap<PassId, Vec<Entity>, NoOpHash>,
}

/// Removes entities from [`SpecializedMaterialPipelineCache`] for the pass based on
/// [`EntitiesNeedingSweep`].
///
/// This runs after all invocations of `early_sweep_entities_needing_specialization`.
/// Because `early_sweep_entities_needing_specialization` is a per-material system and
/// the `SpecializedMaterialPipelineCache` is per-pass, we have to sweep this way.
pub fn late_sweep_entities_needing_specialization<P: MeshPass>(
    views: Query<&ExtractedView, With<P>>,
    mut entities_needing_sweep: ResMut<EntitiesNeedingSweep>,
    mut specialized_material_pipeline_cache: ResMut<SpecializedMaterialPipelineCache<P>>,
) {
    let Some(entities) = entities_needing_sweep.get_mut(&P::id()) else {
        return;
    };
    for view in views {
        let Some(cache) = specialized_material_pipeline_cache.get_mut(&view.retained_view_entity)
        else {
            continue;
        };
        for &entity in entities.iter() {
            cache.remove(&MainEntity::from(entity));
        }
    }
    entities.clear();
}

#[derive(Resource, Deref, DerefMut, Clone, Debug)]
pub struct EntitiesNeedingSpecialization<M> {
    #[deref]
    pub entities: Vec<Entity>,
    _marker: PhantomData<M>,
}

impl<M> Default for EntitiesNeedingSpecialization<M> {
    fn default() -> Self {
        Self {
            entities: Default::default(),
            _marker: Default::default(),
        }
    }
}

/// Stores ticks specifying the last time Bevy specialized the pipelines of each
/// entity.
///
/// Every entity that has a mesh and material must be present in this table,
/// even if that mesh isn't visible.
#[derive(Resource, Deref, DerefMut, Default, Clone, Debug)]
pub struct EntitySpecializationTicks {
    /// A mapping from each main entity to ticks that specify the last time this
    /// entity's pipeline was specialized.
    ///
    /// Every entity that has a mesh and material must be present in this table,
    /// even if that mesh isn't visible.
    #[deref]
    pub entities: MainEntityHashMap<EntitySpecializationTickPair>,
}

/// Ticks that specify the last time an entity's pipeline was specialized.
///
/// We need two different types of ticks here for a subtle reason. First, we
/// need the [`Self::system_tick`], which maps to Bevy's [`SystemChangeTick`],
/// because that's what we use in [`specialize_material_meshes`] to check
/// whether pipelines need specialization. But we also need
/// [`Self::material_instances_tick`], which maps to the
/// [`RenderMaterialInstances::current_change_tick`]. That's because the latter
/// only changes once per frame, which is a guarantee we need to handle the
/// following case:
///
/// 1. The app removes material A from a mesh and replaces it with material B.
///    Both A and B are of different [`Material`] types entirely.
///
/// 2. [`extract_entities_needs_specialization`] runs for material B and marks
///    the mesh as up to date by recording the current tick.
///
/// 3. [`early_sweep_entities_needing_specialization`] runs for material A and checks
///    to ensure it's safe to remove the [`EntitySpecializationTickPair`] for the mesh
///    from the [`EntitySpecializationTicks`]. To do this, it needs to know
///    whether [`extract_entities_needs_specialization`] for some *different*
///    material (in this case, material B) ran earlier in the frame and updated the
///    change tick, and to skip removing the [`EntitySpecializationTickPair`] if so.
///    It can't reliably use the [`Self::system_tick`] to determine this because
///    the [`SystemChangeTick`] can be updated multiple times in the same frame.
///    Instead, it needs a type of tick that's updated only once per frame, after
///    all materials' versions of [`early_sweep_entities_needing_specialization`] have
///    run. The [`RenderMaterialInstances`] tick satisfies this criterion, and so
///    that's what [`early_sweep_entities_needing_specialization`] uses.
#[derive(Clone, Copy, Debug)]
pub struct EntitySpecializationTickPair {
    /// The standard Bevy system tick.
    pub system_tick: Tick,
    /// The tick in [`RenderMaterialInstances`], which is updated in
    /// `late_sweep_material_instances`.
    pub material_instances_tick: Tick,
}

/// Stores the [`SpecializedMaterialViewPipelineCache`] for each view.
#[derive(Resource, Deref, DerefMut)]
pub struct SpecializedMaterialPipelineCache<P> {
    // view entity -> view pipeline cache
    #[deref]
    map: HashMap<RetainedViewEntity, SpecializedMaterialViewPipelineCache>,
    _marker: PhantomData<P>,
}

impl<P> Default for SpecializedMaterialPipelineCache<P> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            _marker: PhantomData,
        }
    }
}

/// Stores the cached render pipeline ID for each entity in a single view, as
/// well as the last time it was changed.
#[derive(Deref, DerefMut, Default)]
pub struct SpecializedMaterialViewPipelineCache {
    // material entity -> (tick, pipeline_id)
    #[deref]
    map: MainEntityHashMap<(Tick, CachedRenderPipelineId)>,
}

pub fn check_entities_needing_specialization<M>(
    needs_specialization: Query<
        Entity,
        (
            Or<(
                Changed<Mesh3d>,
                AssetChanged<Mesh3d>,
                Changed<MeshMaterial3d<M>>,
                AssetChanged<MeshMaterial3d<M>>,
            )>,
            With<MeshMaterial3d<M>>,
        ),
    >,
    mut par_local: Local<Parallel<Vec<Entity>>>,
    mut entities_needing_specialization: ResMut<EntitiesNeedingSpecialization<M>>,
) where
    M: Material,
{
    entities_needing_specialization.clear();

    needs_specialization
        .par_iter()
        .for_each(|entity| par_local.borrow_local_mut().push(entity));

    par_local.drain_into(&mut entities_needing_specialization);
}

pub struct SpecializerKeyContext<'a> {
    pub view_key: MeshPipelineKey,
    pub mesh_pipeline_key: BaseMeshPipelineKey,
    pub mesh_instance_flags: RenderMeshInstanceFlags,
    pub material: &'a PreparedMaterial,
    pub material_asset_id: TypeId,
    pub lightmap: Option<&'a RenderLightmap>,
    pub has_crossfade: bool,
}

/// A trait for creating specializer used by [`specialize_material_meshes`].
pub trait PipelineSpecializer: SpecializedMeshPipeline {
    /// The render pipeline data type used by the corresponding material.
    type Pipeline: Resource;

    /// Creates the key for [`SpecializedMeshPipeline::Key`].
    fn create_key(context: &SpecializerKeyContext) -> Self::Key;

    /// Creates the [`SpecializedMeshPipeline`] for [`SpecializedMeshPipeline::specialize`].
    fn new(pipeline: &Self::Pipeline, material: &PreparedMaterial, pass_id: PassId) -> Self;
}

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
pub struct ViewKeyCache<P>(
    #[deref] HashMap<RetainedViewEntity, MeshPipelineKey>,
    PhantomData<P>,
);

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
pub struct ViewSpecializationTicks<P>(#[deref] HashMap<RetainedViewEntity, Tick>, PhantomData<P>);

#[derive(Resource, Deref, DerefMut, FromWorld)]
pub struct MeshPassSpecializedMeshPipelines<P, S: SpecializedMeshPipeline>(
    #[deref] SpecializedMeshPipelines<S>,
    PhantomData<P>,
);

type Phase1<P> = <<P as MeshPass>::PhaseItems as PhaseItems<P>>::Phase1;
type Phase2<P> = <<P as MeshPass>::PhaseItems as PhaseItems<P>>::Phase2;
type Phase3<P> = <<P as MeshPass>::PhaseItems as PhaseItems<P>>::Phase3;
type Phase4<P> = <<P as MeshPass>::PhaseItems as PhaseItems<P>>::Phase4;

type RenderPhase1<P> = <Phase1<P> as PhaseItemExt>::RenderPhase;
type RenderPhase2<P> = <Phase2<P> as PhaseItemExt>::RenderPhase;
type RenderPhase3<P> = <Phase3<P> as PhaseItemExt>::RenderPhase;
type RenderPhase4<P> = <Phase4<P> as PhaseItemExt>::RenderPhase;

type RenderPhases1<P> = <Phase1<P> as PhaseItemExt>::RenderPhases;
type RenderPhases2<P> = <Phase2<P> as PhaseItemExt>::RenderPhases;
type RenderPhases3<P> = <Phase3<P> as PhaseItemExt>::RenderPhases;
type RenderPhases4<P> = <Phase4<P> as PhaseItemExt>::RenderPhases;

pub fn specialize_material_meshes<P: MeshPass>(
    (view_render_phases_1, view_render_phases_2, view_render_phases_3, view_render_phases_4): (
        Option<Res<RenderPhases1<P>>>,
        Option<Res<RenderPhases2<P>>>,
        Option<Res<RenderPhases3<P>>>,
        Option<Res<RenderPhases4<P>>>,
    ),
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_materials: Res<ErasedRenderAssets<PreparedMaterial>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_material_instances: Res<RenderMaterialInstances>,
    render_lightmaps: Res<RenderLightmaps>,
    render_visibility_ranges: Res<RenderVisibilityRanges>,
    views: Query<(&ExtractedView, &RenderVisibleEntities), With<P>>,
    view_key_cache: Res<ViewKeyCache<P::ViewKeySource>>,
    entity_specialization_ticks: Res<EntitySpecializationTicks>,
    view_specialization_ticks: Res<ViewSpecializationTicks<P::ViewKeySource>>,
    mut specialized_material_pipeline_cache: ResMut<SpecializedMaterialPipelineCache<P>>,
    mut pipelines: ResMut<MeshPassSpecializedMeshPipelines<P, P::Specializer>>,
    pipeline: Res<<P::Specializer as PipelineSpecializer>::Pipeline>,
    pipeline_cache: Res<PipelineCache>,
    ticks: SystemChangeTick,
) where
    <P::Specializer as SpecializedMeshPipeline>::Key: Send + Sync,
{
    // Record the retained IDs of all shadow views so that we can expire old
    // pipeline IDs.
    let mut all_views: HashSet<RetainedViewEntity, FixedHasher> = HashSet::default();

    // All the valid render phase types this pass supports, Dummy phase types are none.
    let valid_render_phase_types = Phase1::<P>::PHASE_TYPES
        | Phase2::<P>::PHASE_TYPES
        | Phase3::<P>::PHASE_TYPES
        | Phase4::<P>::PHASE_TYPES;

    for (view, visible_entities) in &views {
        all_views.insert(view.retained_view_entity);

        // TODO: Move this part to a separate system
        let mut all_invalid = true;
        if let Some(view_render_phases_1) = view_render_phases_1.as_ref() {
            all_invalid &= !view_render_phases_1.contains_key(&view.retained_view_entity);
        }
        if let Some(view_render_phases_2) = view_render_phases_2.as_ref() {
            all_invalid &= !view_render_phases_2.contains_key(&view.retained_view_entity);
        }
        if let Some(view_render_phases_3) = view_render_phases_3.as_ref() {
            all_invalid &= !view_render_phases_3.contains_key(&view.retained_view_entity);
        }
        if let Some(view_render_phases_4) = view_render_phases_4.as_ref() {
            all_invalid &= !view_render_phases_4.contains_key(&view.retained_view_entity);
        }
        if all_invalid {
            continue;
        }

        let Some(view_key) = view_key_cache.get(&view.retained_view_entity) else {
            continue;
        };

        let view_tick = view_specialization_ticks
            .get(&view.retained_view_entity)
            .unwrap();
        let view_specialized_material_pipeline_cache = specialized_material_pipeline_cache
            .entry(view.retained_view_entity)
            .or_default();

        for (_, visible_entity) in visible_entities.iter::<Mesh3d>() {
            let Some(material_instance) = render_material_instances.instances.get(visible_entity)
            else {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                continue;
            };
            let entity_tick = entity_specialization_ticks
                .get(visible_entity)
                .unwrap()
                .system_tick;
            let last_specialized_tick = view_specialized_material_pipeline_cache
                .get(visible_entity)
                .map(|(tick, _)| *tick);
            let needs_specialization = last_specialized_tick.is_none_or(|tick| {
                view_tick.is_newer_than(tick, ticks.this_run())
                    || entity_tick.is_newer_than(tick, ticks.this_run())
            });
            if !needs_specialization {
                continue;
            }
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
            let Some(material) = render_materials.get(material_instance.asset_id) else {
                continue;
            };

            // Current status:
            // - We iterate over all entities even if they are not valid for this pass, which wastes performance.
            // - If an entity's material was valid for this pass previously but becomes invalid in this frame,
            //   we should remove it from `SpecializedMaterialViewPipelineCache`. However we currently lack a way to
            //   detect whether a material is valid for this pass.
            //
            // Two Solutions:
            // - Add pass markers to entities so we can filter them here using `visible_entities.iter::<PassMarker<P>>()`.
            // - Add `material.is_pass_enabled(P::id())` for checking whether the material is valid for this pass.

            // if material
            //     .properties
            //     .get_draw_function(PhaseDrawFunction(P::id()))
            //     .is_none()
            // {
            //     // Prevent cases where the material was valid previously but switched pass during this frame.
            //     view_specialized_material_pipeline_cache.remove(visible_entity);
            //     continue;
            // }

            if !valid_render_phase_types.contains(material.properties.render_phase_type) {
                // Prevent cases where the material was valid previously but switched phase during this frame.
                view_specialized_material_pipeline_cache.remove(visible_entity);
                continue;
            }

            let lightmap = render_lightmaps.render_lightmaps.get(visible_entity);

            let has_crossfade =
                render_visibility_ranges.entity_has_crossfading_visibility_ranges(*visible_entity);

            let key_context = SpecializerKeyContext {
                view_key: *view_key,
                mesh_pipeline_key: mesh.key_bits,
                mesh_instance_flags: mesh_instance.flags,
                material,
                lightmap,
                has_crossfade,
                material_asset_id: material_instance.asset_id.type_id(),
            };

            let key = P::Specializer::create_key(&key_context);

            let material_pipeline_specializer = P::Specializer::new(&pipeline, material, P::id());

            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &material_pipeline_specializer,
                key,
                &mesh.layout,
            );
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            view_specialized_material_pipeline_cache
                .insert(*visible_entity, (ticks.this_run(), pipeline_id));
        }
    }

    // Delete specialized pipelines belonging to views that have expired.
    specialized_material_pipeline_cache
        .retain(|retained_view_entity, _| all_views.contains(retained_view_entity));
}

pub trait RenderPhasePlugin {
    fn new(debug_flags: RenderDebugFlags) -> Self;
}

impl<BPI, GFBD> RenderPhasePlugin for BinnedRenderPhasePlugin<BPI, GFBD>
where
    BPI: BinnedPhaseItem,
    GFBD: GetFullBatchData,
{
    fn new(debug_flags: RenderDebugFlags) -> Self {
        BinnedRenderPhasePlugin::new(debug_flags)
    }
}

impl<SPI, GFBD> RenderPhasePlugin for SortedRenderPhasePlugin<SPI, GFBD>
where
    SPI: SortedPhaseItem,
    GFBD: GetFullBatchData,
{
    fn new(debug_flags: RenderDebugFlags) -> Self {
        SortedRenderPhasePlugin::new(debug_flags)
    }
}

// pub trait PhaseFamily<P: Pass> {
//     type Phase: RenderPhase;
//     type Phases: ViewRenderPhases + Resource;
//     type Plugin: RenderPhasePlugin + Plugin;
// }

pub struct PhaseContext<'a> {
    pub mesh_instance: &'a RenderMeshQueueData<'a>,
    pub material: &'a PreparedMaterial,
    pub mesh_allocator: &'a MeshAllocator,
    pub entity: Entity,
    pub main_entity: MainEntity,
    pub draw_function: DrawFunctionId,
    pub pipeline_id: CachedRenderPipelineId,
    pub current_change_tick: Tick,
    pub gpu_preprocessing_support: GpuPreprocessingSupport,
    pub rangefinder: &'a ViewRangefinder3d,
}

pub trait PhaseItemExt: PhaseItem {
    type RenderPhase: RenderPhase;
    type RenderPhases: ViewRenderPhases<Phase = Self::RenderPhase> + Resource;
    type PhasePlugin: RenderPhasePlugin + Plugin;

    const PHASE_TYPES: RenderPhaseType;

    fn queue(render_phase: &mut Self::RenderPhase, context: &PhaseContext);
}

pub trait RenderPhase {
    fn add(&mut self, context: &PhaseContext);

    fn validate_cached_entity(
        &mut self,
        visible_entity: MainEntity,
        current_change_tick: Tick,
    ) -> bool;
}

pub trait ViewRenderPhases {
    type Phase: RenderPhase;

    fn contains_key(&self, view_entity: &RetainedViewEntity) -> bool;

    fn get_mut(&mut self, view_entity: &RetainedViewEntity) -> Option<&mut Self::Phase>;
}

impl<BPI> RenderPhase for BinnedRenderPhase<BPI>
where
    BPI: BinnedPhaseItem + PhaseItemExt<RenderPhase = BinnedRenderPhase<BPI>>,
{
    #[inline]
    fn add(&mut self, context: &PhaseContext) {
        BPI::queue(self, context);
    }

    #[inline]
    fn validate_cached_entity(
        &mut self,
        visible_entity: MainEntity,
        current_change_tick: Tick,
    ) -> bool {
        self.validate_cached_entity(visible_entity, current_change_tick)
    }
}

impl<SPI> RenderPhase for SortedRenderPhase<SPI>
where
    SPI: SortedPhaseItem + PhaseItemExt<RenderPhase = SortedRenderPhase<SPI>>,
{
    #[inline]
    fn add(&mut self, context: &PhaseContext) {
        SPI::queue(self, context);
    }

    #[inline]
    fn validate_cached_entity(
        &mut self,
        _visible_entity: MainEntity,
        _current_change_tick: Tick,
    ) -> bool {
        false
    }
}

impl<BPI> ViewRenderPhases for ViewBinnedRenderPhases<BPI>
where
    BPI: BinnedPhaseItem + PhaseItemExt<RenderPhase = BinnedRenderPhase<BPI>>,
{
    type Phase = BinnedRenderPhase<BPI>;

    #[inline]
    fn contains_key(&self, view_entity: &RetainedViewEntity) -> bool {
        self.0.contains_key(view_entity)
    }

    #[inline]
    fn get_mut(&mut self, view_entity: &RetainedViewEntity) -> Option<&mut Self::Phase> {
        self.0.get_mut(view_entity)
    }
}

impl<SPI> ViewRenderPhases for ViewSortedRenderPhases<SPI>
where
    SPI: SortedPhaseItem + PhaseItemExt<RenderPhase = SortedRenderPhase<SPI>>,
{
    type Phase = SortedRenderPhase<SPI>;

    #[inline]
    fn contains_key(&self, view_entity: &RetainedViewEntity) -> bool {
        self.0.contains_key(view_entity)
    }

    #[inline]
    fn get_mut(&mut self, view_entity: &RetainedViewEntity) -> Option<&mut Self::Phase> {
        self.0.get_mut(view_entity)
    }
}

/// For each view, iterates over all the meshes visible from that view and adds
/// them to [`BinnedRenderPhase`]s or [`SortedRenderPhase`]s as appropriate.
pub fn queue_material_meshes<P: MeshPass>(
    mut view_render_phases_1: Option<ResMut<RenderPhases1<P>>>,
    mut view_render_phases_2: Option<ResMut<RenderPhases2<P>>>,
    mut view_render_phases_3: Option<ResMut<RenderPhases3<P>>>,
    mut view_render_phases_4: Option<ResMut<RenderPhases4<P>>>,
    render_materials: Res<ErasedRenderAssets<PreparedMaterial>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_material_instances: Res<RenderMaterialInstances>,
    mesh_allocator: Res<MeshAllocator>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    views: Query<(&ExtractedView, &RenderVisibleEntities), With<P>>,
    specialized_material_pipeline_cache: ResMut<SpecializedMaterialPipelineCache<P>>,
) {
    for (view, visible_entities) in &views {
        let mut phase1: Option<&mut RenderPhase1<P>> = view_render_phases_1
            .as_mut()
            .and_then(|view_render_phases| view_render_phases.get_mut(&view.retained_view_entity));
        let mut phase2: Option<&mut RenderPhase2<P>> = view_render_phases_2
            .as_mut()
            .and_then(|view_render_phases| view_render_phases.get_mut(&view.retained_view_entity));
        let mut phase3: Option<&mut RenderPhase3<P>> = view_render_phases_3
            .as_mut()
            .and_then(|view_render_phases| view_render_phases.get_mut(&view.retained_view_entity));
        let mut phase4: Option<&mut RenderPhase4<P>> = view_render_phases_4
            .as_mut()
            .and_then(|view_render_phases| view_render_phases.get_mut(&view.retained_view_entity));

        if phase1.is_none() && phase2.is_none() && phase3.is_none() && phase4.is_none() {
            continue;
        }

        let Some(view_specialized_material_pipeline_cache) =
            specialized_material_pipeline_cache.get(&view.retained_view_entity)
        else {
            continue;
        };

        let rangefinder = view.rangefinder3d();

        for (render_entity, visible_entity) in visible_entities.iter::<Mesh3d>() {
            let Some((current_change_tick, pipeline_id)) = view_specialized_material_pipeline_cache
                .get(visible_entity)
                .map(|(current_change_tick, pipeline_id)| (*current_change_tick, *pipeline_id))
            else {
                continue;
            };

            // Skip the entity if it's cached in a bin and up to date.
            // NOTE: SortedRenderPhase will always return false.
            let mut any_cached = false;
            if let Some(phase1) = phase1.as_mut() {
                any_cached |= phase1.validate_cached_entity(*visible_entity, current_change_tick);
            }
            if let Some(phase2) = phase2.as_mut() {
                any_cached |= phase2.validate_cached_entity(*visible_entity, current_change_tick);
            }
            if let Some(phase3) = phase3.as_mut() {
                any_cached |= phase3.validate_cached_entity(*visible_entity, current_change_tick);
            }
            if let Some(phase4) = phase4.as_mut() {
                any_cached |= phase4.validate_cached_entity(*visible_entity, current_change_tick);
            }
            if any_cached {
                continue;
            }

            let Some(material_instance) = render_material_instances.instances.get(visible_entity)
            else {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                continue;
            };
            let Some(material) = render_materials.get(material_instance.asset_id) else {
                continue;
            };

            // NOTE: Because one pass could have phases with same phase types,
            // we don't use mutual exclusion conditions here.
            let phase_type = material.properties.render_phase_type;
            if Phase1::<P>::PHASE_TYPES.contains(phase_type)
                && let Some(phase1) = phase1.as_mut()
                && let Some(draw_function) = material
                    .properties
                    .get_draw_function(Phase1DrawFunction(P::id()))
            {
                let context = PhaseContext {
                    mesh_instance: &mesh_instance,
                    material,
                    mesh_allocator: &mesh_allocator,
                    entity: *render_entity,
                    main_entity: *visible_entity,
                    draw_function,
                    pipeline_id,
                    current_change_tick,
                    gpu_preprocessing_support: *gpu_preprocessing_support,
                    rangefinder: &rangefinder,
                };
                phase1.add(&context);
            }
            if Phase2::<P>::PHASE_TYPES.contains(phase_type)
                && let Some(phase2) = phase2.as_mut()
                && let Some(draw_function) = material
                    .properties
                    .get_draw_function(Phase2DrawFunction(P::id()))
            {
                let context = PhaseContext {
                    mesh_instance: &mesh_instance,
                    material,
                    mesh_allocator: &mesh_allocator,
                    entity: *render_entity,
                    main_entity: *visible_entity,
                    draw_function,
                    pipeline_id,
                    current_change_tick,
                    gpu_preprocessing_support: *gpu_preprocessing_support,
                    rangefinder: &rangefinder,
                };
                phase2.add(&context);
            }
            if Phase3::<P>::PHASE_TYPES.contains(phase_type)
                && let Some(phase3) = phase3.as_mut()
                && let Some(draw_function) = material
                    .properties
                    .get_draw_function(Phase3DrawFunction(P::id()))
            {
                let context = PhaseContext {
                    mesh_instance: &mesh_instance,
                    material,
                    mesh_allocator: &mesh_allocator,
                    entity: *render_entity,
                    main_entity: *visible_entity,
                    draw_function,
                    pipeline_id,
                    current_change_tick,
                    gpu_preprocessing_support: *gpu_preprocessing_support,
                    rangefinder: &rangefinder,
                };
                phase3.add(&context);
            }
            if Phase4::<P>::PHASE_TYPES.contains(phase_type)
                && let Some(phase4) = phase4.as_mut()
                && let Some(draw_function) = material
                    .properties
                    .get_draw_function(Phase4DrawFunction(P::id()))
            {
                let context = PhaseContext {
                    mesh_instance: &mesh_instance,
                    material,
                    mesh_allocator: &mesh_allocator,
                    entity: *render_entity,
                    main_entity: *visible_entity,
                    draw_function,
                    pipeline_id,
                    current_change_tick,
                    gpu_preprocessing_support: *gpu_preprocessing_support,
                    rangefinder: &rangefinder,
                };
                phase4.add(&context);
            };
        }
    }
}

/// Default render method used for opaque materials.
#[derive(Default, Resource, Clone, Debug, ExtractResource, Reflect)]
#[reflect(Resource, Default, Debug, Clone)]
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
#[reflect(Default, Clone, PartialEq)]
pub enum OpaqueRendererMethod {
    #[default]
    Forward,
    Deferred,
    Auto,
}

#[derive(ShaderLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct MaterialVertexShader(pub PassId);

#[derive(ShaderLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct MaterialFragmentShader(pub PassId);

#[derive(ShaderLabel, Debug, Hash, PartialEq, Eq, Clone, Default)]
pub struct MeshletFragmentShader;

#[derive(ShaderLabel, Debug, Hash, PartialEq, Eq, Clone, Default)]
pub struct MeshletPrepassFragmentShader;

#[derive(ShaderLabel, Debug, Hash, PartialEq, Eq, Clone, Default)]
pub struct MeshletDeferredFragmentShader;

#[derive(DrawFunctionLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct Phase1DrawFunction(pub PassId);

#[derive(DrawFunctionLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct Phase2DrawFunction(pub PassId);

#[derive(DrawFunctionLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct Phase3DrawFunction(pub PassId);

#[derive(DrawFunctionLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct Phase4DrawFunction(pub PassId);

#[derive(DrawFunctionLabel, Debug, Hash, PartialEq, Eq, Clone, Default)]
pub struct ShadowsDrawFunction;

#[derive(Debug)]
pub struct ErasedMaterialKey {
    type_id: TypeId,
    hash: u64,
    value: Box<dyn Any + Send + Sync>,
    vtable: Arc<ErasedMaterialKeyVTable>,
}

#[derive(Debug)]
pub struct ErasedMaterialKeyVTable {
    clone_fn: fn(&dyn Any) -> Box<dyn Any + Send + Sync>,
    partial_eq_fn: fn(&dyn Any, &dyn Any) -> bool,
}

impl ErasedMaterialKey {
    pub fn new<T>(material_key: T) -> Self
    where
        T: Clone + Hash + PartialEq + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let hash = FixedHasher::hash_one(&FixedHasher, &material_key);

        fn clone<T: Clone + Send + Sync + 'static>(any: &dyn Any) -> Box<dyn Any + Send + Sync> {
            Box::new(any.downcast_ref::<T>().unwrap().clone())
        }
        fn partial_eq<T: PartialEq + 'static>(a: &dyn Any, b: &dyn Any) -> bool {
            a.downcast_ref::<T>().unwrap() == b.downcast_ref::<T>().unwrap()
        }

        Self {
            type_id,
            hash,
            value: Box::new(material_key),
            vtable: Arc::new(ErasedMaterialKeyVTable {
                clone_fn: clone::<T>,
                partial_eq_fn: partial_eq::<T>,
            }),
        }
    }

    pub fn to_key<T: Clone + 'static>(&self) -> T {
        debug_assert_eq!(self.type_id, TypeId::of::<T>());
        self.value.downcast_ref::<T>().unwrap().clone()
    }
}

impl PartialEq for ErasedMaterialKey {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
            && (self.vtable.partial_eq_fn)(self.value.as_ref(), other.value.as_ref())
    }
}

impl Eq for ErasedMaterialKey {}

impl Clone for ErasedMaterialKey {
    fn clone(&self) -> Self {
        Self {
            type_id: self.type_id,
            hash: self.hash,
            value: (self.vtable.clone_fn)(self.value.as_ref()),
            vtable: self.vtable.clone(),
        }
    }
}

impl Hash for ErasedMaterialKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        self.hash.hash(state);
    }
}

impl Default for ErasedMaterialKey {
    fn default() -> Self {
        Self::new(())
    }
}

/// Common [`Material`] properties, calculated for a specific material instance.
#[derive(Default)]
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
    /// This allows taking color output from the [`bevy_core_pipeline::core_3d::Opaque3d`] pass as an input, (for screen-space transmission) but requires
    /// rendering to take place in a separate [`bevy_core_pipeline::core_3d::Transmissive3d`] pass.
    pub reads_view_transmission_texture: bool,
    pub render_phase_type: RenderPhaseType,
    pub material_layout: Option<BindGroupLayoutDescriptor>,
    /// Backing array is a size of 4 because the `StandardMaterial` needs 4 draw functions by default
    pub draw_functions: SmallVec<[(InternedDrawFunctionLabel, DrawFunctionId); 4]>,
    /// Backing array is a size of 3 because the `StandardMaterial` has 3 custom shaders (`frag`, `prepass_frag`, `deferred_frag`) which is the
    /// most common use case
    pub shaders: SmallVec<[(InternedShaderLabel, Handle<Shader>); 3]>,
    /// Whether this material *actually* uses bindless resources, taking the
    /// platform support (or lack thereof) of bindless resources into account.
    pub bindless: bool,
    pub specialize: Option<
        fn(
            &MaterialPipeline,
            &mut RenderPipelineDescriptor,
            &MeshVertexBufferLayoutRef,
            ErasedMaterialPipelineKey,
            // PassId,
        ) -> Result<(), SpecializedMeshPipelineError>,
    >,
    /// The key for this material, typically a bitfield of flags that are used to modify
    /// the pipeline descriptor used for this material.
    pub material_key: ErasedMaterialKey,
    /// Whether shadows are enabled for this material
    pub shadows_enabled: bool,
    /// Whether prepass is enabled for this material
    pub prepass_enabled: bool,
}

impl MaterialProperties {
    pub fn get_shader(&self, label: impl ShaderLabel) -> Option<Handle<Shader>> {
        self.shaders
            .iter()
            .find(|(inner_label, _)| inner_label == &label.intern())
            .map(|(_, shader)| shader)
            .cloned()
    }

    pub fn add_shader(&mut self, label: impl ShaderLabel, shader: Handle<Shader>) {
        self.shaders.push((label.intern(), shader));
    }

    pub fn get_draw_function(&self, label: impl DrawFunctionLabel) -> Option<DrawFunctionId> {
        self.draw_functions
            .iter()
            .find(|(inner_label, _)| inner_label == &label.intern())
            .map(|(_, shader)| shader)
            .cloned()
    }

    pub fn add_draw_function(
        &mut self,
        label: impl DrawFunctionLabel,
        draw_function: DrawFunctionId,
    ) {
        self.draw_functions.push((label.intern(), draw_function));
    }
}

bitflags::bitflags! {
    // NOTE: To handle the case like `Shadow` where a single phase works for materials
    // with different render phase types, we use bitflags instead of enum.

    /// Defines all the possible render phase types for a material.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct RenderPhaseType: u8 {
        const Opaque =       1 << 0;
        const AlphaMask =    1 << 1;
        const Transmissive = 1 << 2;
        const Transparent =  1 << 3;
    }
}

impl Default for RenderPhaseType {
    fn default() -> Self {
        RenderPhaseType::Opaque
    }
}

/// Stores draw functions for each `MeshPass`.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct PassPhaseDrawFunctions(HashMap<PassId, PhaseDrawFunctions, NoOpHash>);

/// Stores the draw functions for each `PhaseItem` of a `MeshPass`.
#[derive(Default, Deref, DerefMut)]
pub struct PhaseDrawFunctions([Option<DrawFunctionId>; PASS_MAX_PHASES]);

/// A resource that maps each untyped material ID to its binding.
///
/// This duplicates information in `RenderAssets<M>`, but it doesn't have the
/// `M` type parameter, so it can be used in untyped contexts like
/// [`crate::render::mesh::collect_meshes_for_gpu_building`].
#[derive(Resource, Default, Deref, DerefMut)]
pub struct RenderMaterialBindings(HashMap<UntypedAssetId, MaterialBindingId>);

/// Data prepared for a [`Material`] instance.
pub struct PreparedMaterial {
    pub binding: MaterialBindingId,
    pub properties: Arc<MaterialProperties>,
}

// orphan rules T_T
impl<M: Material> ErasedRenderAsset for MeshMaterial3d<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type SourceAsset = M;
    type ErasedAsset = PreparedMaterial;

    type Param = (
        SRes<RenderDevice>,
        SRes<PipelineCache>,
        SRes<DefaultOpaqueRendererMethod>,
        SResMut<MaterialBindGroupAllocators>,
        SResMut<RenderMaterialBindings>,
        SRes<DrawFunctions<Shadow>>,
        SRes<PassPhaseDrawFunctions>,
        SRes<AssetServer>,
        M::Param,
    );

    fn prepare_asset(
        material: Self::SourceAsset,
        material_id: AssetId<Self::SourceAsset>,
        (
            render_device,
            pipeline_cache,
            default_opaque_render_method,
            bind_group_allocators,
            render_material_bindings,
            shadow_draw_functions,
            pass_phase_draw_functions,
            asset_server,
            material_param,
        ): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::ErasedAsset, PrepareAssetError<Self::SourceAsset>> {
        let shadows_enabled = M::enable_shadows();
        let prepass_enabled = M::enable_prepass();

        let draw_shadows = shadow_draw_functions.read().id::<DrawPrepass>();

        let mut draw_functions =
            SmallVec::from_iter([(ShadowsDrawFunction.intern(), draw_shadows)]);

        let render_method = match material.opaque_render_method() {
            OpaqueRendererMethod::Forward => OpaqueRendererMethod::Forward,
            OpaqueRendererMethod::Deferred => OpaqueRendererMethod::Deferred,
            OpaqueRendererMethod::Auto => default_opaque_render_method.0,
        };

        let mut mesh_pipeline_key_bits = MeshPipelineKey::empty();
        mesh_pipeline_key_bits.set(
            MeshPipelineKey::READS_VIEW_TRANSMISSION_TEXTURE,
            material.reads_view_transmission_texture(),
        );

        let reads_view_transmission_texture =
            mesh_pipeline_key_bits.contains(MeshPipelineKey::READS_VIEW_TRANSMISSION_TEXTURE);

        let render_phase_type =
            alpha_mode_render_phase_type(material.alpha_mode(), reads_view_transmission_texture);

        let mut shaders = SmallVec::new();
        let mut add_shader = |label: InternedShaderLabel, shader_ref: ShaderRef| {
            let mayber_shader = match shader_ref {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            };
            if let Some(shader) = mayber_shader {
                shaders.push((label, shader));
            }
        };

        for (
            pass_id,
            ShaderSet {
                mut vertex,
                mut fragment,
            },
        ) in M::shaders()
        {
            let Some(phase_draw_functions) = pass_phase_draw_functions.get(&pass_id) else {
                continue;
            };

            const PHASE_DRAW_FUNCTIONS_ERROR: &str = "The index here should never be out of bounds";

            if let Some(draw_function) = phase_draw_functions
                .first()
                .expect(PHASE_DRAW_FUNCTIONS_ERROR)
            {
                draw_functions.push((Phase1DrawFunction(pass_id).intern(), *draw_function));
            };

            if let Some(draw_function) = phase_draw_functions
                .get(1)
                .expect(PHASE_DRAW_FUNCTIONS_ERROR)
            {
                draw_functions.push((Phase2DrawFunction(pass_id).intern(), *draw_function));
            };

            if let Some(draw_function) = phase_draw_functions
                .get(2)
                .expect(PHASE_DRAW_FUNCTIONS_ERROR)
            {
                draw_functions.push((Phase3DrawFunction(pass_id).intern(), *draw_function));
            };

            if let Some(draw_function) = phase_draw_functions
                .get(3)
                .expect(PHASE_DRAW_FUNCTIONS_ERROR)
            {
                draw_functions.push((Phase4DrawFunction(pass_id).intern(), *draw_function));
            };

            // If users are still using the traditional method, it continues to be used until they migrate.
            if pass_id == Prepass::id() {
                let prepass_vertex = M::prepass_vertex_shader();
                if !matches!(prepass_vertex, ShaderRef::Default) {
                    vertex = prepass_vertex;
                }

                let prepass_fragment = M::prepass_fragment_shader();
                if !matches!(prepass_fragment, ShaderRef::Default) {
                    fragment = prepass_fragment;
                }
            }

            if pass_id == DeferredPass::id() {
                let deferred_vertex = M::deferred_vertex_shader();
                if !matches!(deferred_vertex, ShaderRef::Default) {
                    vertex = deferred_vertex;
                }

                let deferred_fragment = M::deferred_fragment_shader();
                if !matches!(deferred_fragment, ShaderRef::Default) {
                    fragment = deferred_fragment;
                }
            }

            if pass_id == MainPass::id() {
                let main_pass_vertex = M::vertex_shader();
                if !matches!(main_pass_vertex, ShaderRef::Default) {
                    vertex = main_pass_vertex;
                }

                let main_pass_fragment = M::fragment_shader();
                if !matches!(main_pass_fragment, ShaderRef::Default) {
                    fragment = main_pass_fragment;
                }
            }

            add_shader(MaterialVertexShader(pass_id).intern(), vertex);
            add_shader(MaterialFragmentShader(pass_id).intern(), fragment);
        }

        #[cfg(feature = "meshlet")]
        {
            add_shader(
                MeshletFragmentShader.intern(),
                M::meshlet_mesh_fragment_shader(),
            );
            add_shader(
                MeshletPrepassFragmentShader.intern(),
                M::meshlet_mesh_prepass_fragment_shader(),
            );
            add_shader(
                MeshletDeferredFragmentShader.intern(),
                M::meshlet_mesh_deferred_fragment_shader(),
            );
        }

        let bindless = material_uses_bindless_resources::<M>(render_device);
        let bind_group_data = material.bind_group_data();
        let material_key = ErasedMaterialKey::new(bind_group_data);
        fn specialize<M: Material>(
            pipeline: &MaterialPipeline,
            descriptor: &mut RenderPipelineDescriptor,
            mesh_layout: &MeshVertexBufferLayoutRef,
            erased_key: ErasedMaterialPipelineKey,
            // pass_id: PassId,
        ) -> Result<(), SpecializedMeshPipelineError>
        where
            M::Data: Hash + Clone,
        {
            let material_key = erased_key.material_key.to_key();
            M::specialize(
                pipeline,
                descriptor,
                mesh_layout,
                MaterialPipelineKey {
                    mesh_key: erased_key.mesh_key,
                    bind_group_data: material_key,
                },
                // pass_id,
            )
        }

        let material_layout = M::bind_group_layout_descriptor(render_device);
        let actual_material_layout = pipeline_cache.get_bind_group_layout(&material_layout);

        match material.unprepared_bind_group(
            &actual_material_layout,
            render_device,
            material_param,
            false,
        ) {
            Ok(unprepared) => {
                let bind_group_allocator =
                    bind_group_allocators.get_mut(&TypeId::of::<M>()).unwrap();
                // Allocate or update the material.
                let binding = match render_material_bindings.entry(material_id.into()) {
                    Entry::Occupied(mut occupied_entry) => {
                        // TODO: Have a fast path that doesn't require
                        // recreating the bind group if only buffer contents
                        // change. For now, we just delete and recreate the bind
                        // group.
                        bind_group_allocator.free(*occupied_entry.get());
                        let new_binding =
                            bind_group_allocator.allocate_unprepared(unprepared, &material_layout);
                        *occupied_entry.get_mut() = new_binding;
                        new_binding
                    }
                    Entry::Vacant(vacant_entry) => *vacant_entry.insert(
                        bind_group_allocator.allocate_unprepared(unprepared, &material_layout),
                    ),
                };

                Ok(PreparedMaterial {
                    binding,
                    properties: Arc::new(MaterialProperties {
                        alpha_mode: material.alpha_mode(),
                        depth_bias: material.depth_bias(),
                        reads_view_transmission_texture,
                        render_phase_type,
                        render_method,
                        mesh_pipeline_key_bits,
                        material_layout: Some(material_layout),
                        draw_functions,
                        shaders,
                        bindless,
                        specialize: Some(specialize::<M>),
                        material_key,
                        shadows_enabled,
                        prepass_enabled,
                    }),
                })
            }

            Err(AsBindGroupError::RetryNextUpdate) => {
                Err(PrepareAssetError::RetryNextUpdate(material))
            }

            Err(AsBindGroupError::CreateBindGroupDirectly) => {
                // This material has opted out of automatic bind group creation
                // and is requesting a fully-custom bind group. Invoke
                // `as_bind_group` as requested, and store the resulting bind
                // group in the slot.
                match material.as_bind_group(
                    &material_layout,
                    render_device,
                    pipeline_cache,
                    material_param,
                ) {
                    Ok(prepared_bind_group) => {
                        let bind_group_allocator =
                            bind_group_allocators.get_mut(&TypeId::of::<M>()).unwrap();
                        // Store the resulting bind group directly in the slot.
                        let material_binding_id =
                            bind_group_allocator.allocate_prepared(prepared_bind_group);
                        render_material_bindings.insert(material_id.into(), material_binding_id);

                        Ok(PreparedMaterial {
                            binding: material_binding_id,
                            properties: Arc::new(MaterialProperties {
                                alpha_mode: material.alpha_mode(),
                                depth_bias: material.depth_bias(),
                                reads_view_transmission_texture,
                                render_phase_type,
                                render_method,
                                mesh_pipeline_key_bits,
                                material_layout: Some(material_layout),
                                draw_functions,
                                shaders,
                                bindless,
                                specialize: Some(specialize::<M>),
                                material_key,
                                shadows_enabled,
                                prepass_enabled,
                            }),
                        })
                    }

                    Err(AsBindGroupError::RetryNextUpdate) => {
                        Err(PrepareAssetError::RetryNextUpdate(material))
                    }

                    Err(other) => Err(PrepareAssetError::AsBindGroupError(other)),
                }
            }

            Err(other) => Err(PrepareAssetError::AsBindGroupError(other)),
        }
    }

    fn unload_asset(
        source_asset: AssetId<Self::SourceAsset>,
        (_, _, _, bind_group_allocators, render_material_bindings, ..): &mut SystemParamItem<
            Self::Param,
        >,
    ) {
        let Some(material_binding_id) = render_material_bindings.remove(&source_asset.untyped())
        else {
            return;
        };
        let bind_group_allactor = bind_group_allocators.get_mut(&TypeId::of::<M>()).unwrap();
        bind_group_allactor.free(material_binding_id);
    }
}

/// Creates and/or recreates any bind groups that contain materials that were
/// modified this frame.
pub fn prepare_material_bind_groups(
    mut allocators: ResMut<MaterialBindGroupAllocators>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    fallback_image: Res<FallbackImage>,
    fallback_resources: Res<FallbackBindlessResources>,
) {
    for (_, allocator) in allocators.iter_mut() {
        allocator.prepare_bind_groups(
            &render_device,
            &pipeline_cache,
            &fallback_resources,
            &fallback_image,
        );
    }
}

/// Uploads the contents of all buffers that the [`MaterialBindGroupAllocator`]
/// manages to the GPU.
///
/// Non-bindless allocators don't currently manage any buffers, so this method
/// only has an effect for bindless allocators.
pub fn write_material_bind_group_buffers(
    mut allocators: ResMut<MaterialBindGroupAllocators>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    for (_, allocator) in allocators.iter_mut() {
        allocator.write_buffers(&render_device, &render_queue);
    }
}
