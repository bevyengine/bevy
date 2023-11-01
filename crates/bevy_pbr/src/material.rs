use crate::*;
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp, AssetEvent, AssetId, AssetServer, Assets, Handle};
use bevy_core_pipeline::core_3d::{
    AlphaMask3d, Camera3d, Opaque3d, ScreenSpaceTransmissionQuality, Transmissive3d, Transparent3d,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::*,
    system::{
        lifetimeless::{Read, SRes},
        SystemParamItem,
    },
};
use bevy_reflect::Reflect;
use bevy_render::{
    extract_instances::{ExtractInstancesPlugin, ExtractedInstances},
    extract_resource::ExtractResource,
    mesh::{Mesh, MeshKey, MeshVertexBufferLayout},
    pipeline_keys::{KeyRepack, KeyShaderDefs, PackedPipelineKey, PipelineKeys, SystemKey},
    prelude::Image,
    render_asset::{prepare_assets, RenderAssets},
    render_phase::*,
    render_resource::*,
    renderer::RenderDevice,
    texture::FallbackImage,
    view::{ExtractedView, VisibleEntities},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_utils::{tracing::error, HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

/// Materials are used alongside [`MaterialPlugin`] and [`MaterialMeshBundle`]
/// to spawn entities that are rendered with a specific [`Material`] type. They serve as an easy to use high level
/// way to render [`Mesh`] entities with custom shader logic.
///
/// Materials must implement [`AsBindGroup`] to define how data will be transferred to the GPU and bound in shaders.
/// [`AsBindGroup`] can be derived, which makes generating bindings straightforward. See the [`AsBindGroup`] docs for details.
///
/// # Example
///
/// Here is a simple Material implementation. The [`AsBindGroup`] derive has many features. To see what else is available,
/// check out the [`AsBindGroup`] documentation.
/// ```
/// # use bevy_pbr::{Material, MaterialMeshBundle};
/// # use bevy_ecs::prelude::*;
/// # use bevy_reflect::{TypeUuid, TypePath};
/// # use bevy_render::{render_resource::{AsBindGroup, ShaderRef}, texture::Image, color::Color};
/// # use bevy_asset::{Handle, AssetServer, Assets, Asset};
///
/// #[derive(AsBindGroup, Debug, Clone, Asset, TypePath)]
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
/// @group(1) @binding(0) var<uniform> color: vec4<f32>;
/// @group(1) @binding(1) var color_texture: texture_2d<f32>;
/// @group(1) @binding(2) var color_sampler: sampler;
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

    /// Customizes the default [`RenderPipelineDescriptor`] for a specific entity using the entity's
    /// [`MaterialPipelineKey`] and [`MeshVertexBufferLayout`] as input.
    #[allow(unused_variables)]
    #[inline]
    fn specialize(
        pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
        key: PipelineKey<NewMaterialPipelineKey<Self>>,
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
        app.init_asset::<M>()
            .add_plugins(ExtractInstancesPlugin::<AssetId<M>>::extract_visible());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<DrawFunctions<Shadow>>()
                .add_render_command::<Shadow, DrawPrepass<M>>()
                .add_render_command::<Transmissive3d, DrawMaterial<M>>()
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
                            .in_set(RenderSet::PrepareAssets)
                            .after(prepare_assets::<Image>),
                        render::queue_shadows::<M>
                            .in_set(RenderSet::QueueMeshes)
                            .after(prepare_materials::<M>),
                        queue_material_meshes::<M>
                            .in_set(RenderSet::QueueMeshes)
                            .after(prepare_materials::<M>),
                    ),
                )
                .register_key::<AlphaKey>();
        }

        // PrepassPipelinePlugin is required for shadow mapping and the optional PrepassPlugin
        app.add_plugins(PrepassPipelinePlugin::<M>::default());

        if self.prepass_enabled {
            app.add_plugins(PrepassPlugin::<M>::default());
        }
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<MaterialPipeline<M>>();
        }
    }
}

#[derive(PipelineKey)]
pub struct NewMaterialPipelineKey<M: Material> {
    pub view_key: PbrViewKey,
    pub mesh_key: MeshKey,
    pub material_key: NewMaterialKey<M>,
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
    type Key = NewMaterialPipelineKey<M>;

    fn specialize(
        &self,
        key: PipelineKey<Self::Key>,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mesh_pipeline_key = key.construct(NewMeshPipelineKey {
            view_key: key.view_key,
            mesh_key: key.mesh_key,
            alpha_mode: key.material_key.alpha,
        });
        let mut descriptor = self.mesh_pipeline.specialize(mesh_pipeline_key, layout)?;
        // add shader defs for the full key (including non-MeshPipelineKey parts of the key)
        descriptor.vertex.shader_defs.extend(key.shader_defs());
        descriptor
            .fragment
            .as_mut()
            .map(|frag| frag.shader_defs.extend(key.shader_defs()));
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
    type Param = (SRes<RenderMaterials<M>>, SRes<RenderMaterialInstances<M>>);
    type ViewWorldQuery = ();
    type ItemWorldQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: (),
        (materials, material_instances): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let materials = materials.into_inner();
        let material_instances = material_instances.into_inner();

        let Some(material_asset_id) = material_instances.get(&item.entity()) else {
            return RenderCommandResult::Failure;
        };
        let Some(material) = materials.get(material_asset_id) else {
            return RenderCommandResult::Failure;
        };
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub type RenderMaterialInstances<M> = ExtractedInstances<AssetId<M>>;

#[allow(clippy::too_many_arguments)]
pub fn queue_material_meshes<M: Material>(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transmissive_draw_functions: Res<DrawFunctions<Transmissive3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    material_pipeline: Res<MaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<MaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderMaterials<M>>,
    mut render_mesh_instances: ResMut<RenderMeshInstances>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    mut views: Query<(
        &PipelineKeys,
        &ExtractedView,
        &VisibleEntities,
        &mut RenderPhase<Opaque3d>,
        &mut RenderPhase<AlphaMask3d>,
        &mut RenderPhase<Transmissive3d>,
        &mut RenderPhase<Transparent3d>,
    )>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    for (
        keys,
        view,
        visible_entities,
        mut opaque_phase,
        mut alpha_mask_phase,
        mut transmissive_phase,
        mut transparent_phase,
    ) in &mut views
    {
        let draw_opaque_pbr = opaque_draw_functions.read().id::<DrawMaterial<M>>();
        let draw_alpha_mask_pbr = alpha_mask_draw_functions.read().id::<DrawMaterial<M>>();
        let draw_transmissive_pbr = transmissive_draw_functions.read().id::<DrawMaterial<M>>();
        let draw_transparent_pbr = transparent_draw_functions.read().id::<DrawMaterial<M>>();

        let Some(view_key_new) = keys.get_packed_key::<PbrViewKey>() else {
            continue;
        };

        let rangefinder = view.rangefinder3d();

        for visible_entity in &visible_entities.entities {
            let Some(material_asset_id) = render_material_instances.get(visible_entity) else {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.get_mut(visible_entity) else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
            let Some(material) = render_materials.get(material_asset_id) else {
                continue;
            };

            let forward = match material.properties.render_method {
                OpaqueRendererMethod::Forward => true,
                OpaqueRendererMethod::Deferred => false,
                OpaqueRendererMethod::Auto => unreachable!(),
            };

            let mesh_key_new = render_meshes
                .get(mesh_instance.mesh_asset_id)
                .unwrap()
                .packed_key;
            let material_key_new = render_materials
                .get(material_asset_id)
                .unwrap()
                .new_packed_key;

            let composite_key =
                NewMaterialPipelineKey::repack((view_key_new, mesh_key_new, material_key_new));

            let pipeline_id = pipelines.specialize(
                &pipeline_cache,
                &material_pipeline,
                composite_key,
                &mesh.layout,
            );
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            mesh_instance.material_bind_group_id = material.get_bind_group_id();

            let distance = rangefinder
                .distance_translation(&mesh_instance.transforms.transform.translation)
                + material.properties.depth_bias;
            match material.properties.alpha_mode {
                AlphaMode::Opaque => {
                    if material.properties.reads_view_transmission_texture {
                        transmissive_phase.add(Transmissive3d {
                            entity: *visible_entity,
                            draw_function: draw_transmissive_pbr,
                            pipeline: pipeline_id,
                            distance,
                            batch_range: 0..1,
                            dynamic_offset: None,
                        });
                    } else if forward {
                        opaque_phase.add(Opaque3d {
                            entity: *visible_entity,
                            draw_function: draw_opaque_pbr,
                            pipeline: pipeline_id,
                            distance,
                            batch_range: 0..1,
                            dynamic_offset: None,
                        });
                    }
                }
                AlphaMode::Mask(_) => {
                    if material.properties.reads_view_transmission_texture {
                        transmissive_phase.add(Transmissive3d {
                            entity: *visible_entity,
                            draw_function: draw_transmissive_pbr,
                            pipeline: pipeline_id,
                            distance,
                            batch_range: 0..1,
                            dynamic_offset: None,
                        });
                    } else if forward {
                        alpha_mask_phase.add(AlphaMask3d {
                            entity: *visible_entity,
                            draw_function: draw_alpha_mask_pbr,
                            pipeline: pipeline_id,
                            distance,
                            batch_range: 0..1,
                            dynamic_offset: None,
                        });
                    }
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
                        batch_range: 0..1,
                        dynamic_offset: None,
                    });
                }
            }
        }
    }
}

/// Default render method used for opaque materials.
#[derive(Default, Resource, Clone, Debug, ExtractResource, Reflect)]
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
#[derive(Default, Clone, Copy, Debug, Reflect)]
pub enum OpaqueRendererMethod {
    #[default]
    Forward,
    Deferred,
    Auto,
}

/// Common [`Material`] properties, calculated for a specific material instance.
pub struct MaterialProperties {
    /// Is this material should be rendered by the deferred renderer when.
    /// AlphaMode::Opaque or AlphaMode::Mask
    pub render_method: OpaqueRendererMethod,
    /// The [`AlphaMode`] of this material.
    pub alpha_mode: AlphaMode,
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
    pub new_packed_key: PackedPipelineKey<NewMaterialKey<T>>,
    pub properties: MaterialProperties,
}

#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Deref, DerefMut)]
pub struct MaterialBindGroupId(Option<BindGroupId>);

impl<T: Material> PreparedMaterial<T> {
    pub fn get_bind_group_id(&self) -> MaterialBindGroupId {
        MaterialBindGroupId(Some(self.bind_group.id()))
    }
}

#[derive(Resource)]
pub struct ExtractedMaterials<M: Material> {
    extracted: Vec<(AssetId<M>, M)>,
    removed: Vec<AssetId<M>>,
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
pub struct RenderMaterials<T: Material>(pub HashMap<AssetId<T>, PreparedMaterial<T>>);

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
    for event in events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                changed_assets.insert(*id);
            }
            AssetEvent::Removed { id } => {
                changed_assets.remove(id);
                removed.push(*id);
            }
            AssetEvent::LoadedWithDependencies { .. } => {
                // TODO: handle this
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for id in changed_assets.drain() {
        if let Some(asset) = assets.get(id) {
            extracted_assets.push((id, asset.clone()));
        }
    }

    commands.insert_resource(ExtractedMaterials {
        extracted: extracted_assets,
        removed,
    });
}

/// All [`Material`] values of a given type that should be prepared next frame.
pub struct PrepareNextFrameMaterials<M: Material> {
    assets: Vec<(AssetId<M>, M)>,
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
#[allow(clippy::too_many_arguments)]
pub fn prepare_materials<M: Material>(
    mut prepare_next_frame: Local<PrepareNextFrameMaterials<M>>,
    mut extracted_assets: ResMut<ExtractedMaterials<M>>,
    mut render_materials: ResMut<RenderMaterials<M>>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    pipeline: Res<MaterialPipeline<M>>,
    default_opaque_render_method: Res<DefaultOpaqueRendererMethod>,
    pipeline_cache: Res<PipelineCache>,
) {
    let queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (id, material) in queued_assets.into_iter() {
        match prepare_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
            default_opaque_render_method.0,
            &pipeline_cache,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(id, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((id, material));
            }
        }
    }

    for removed in std::mem::take(&mut extracted_assets.removed) {
        render_materials.remove(&removed);
    }

    for (id, material) in std::mem::take(&mut extracted_assets.extracted) {
        match prepare_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
            default_opaque_render_method.0,
            &pipeline_cache,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(id, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((id, material));
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
    default_opaque_render_method: OpaqueRendererMethod,
    pipeline_cache: &PipelineCache,
) -> Result<PreparedMaterial<M>, AsBindGroupError> {
    let prepared = material.as_bind_group(
        &pipeline.material_layout,
        render_device,
        images,
        fallback_image,
    )?;
    let method = match material.opaque_render_method() {
        OpaqueRendererMethod::Forward => OpaqueRendererMethod::Forward,
        OpaqueRendererMethod::Deferred => OpaqueRendererMethod::Deferred,
        OpaqueRendererMethod::Auto => default_opaque_render_method,
    };
    let key = NewMaterialKey {
        alpha: material.alpha_mode().into(),
        opaque_method: match method {
            OpaqueRendererMethod::Forward => OpaqueMethodKey::Forward,
            OpaqueRendererMethod::Deferred => OpaqueMethodKey::Deferred,
            OpaqueRendererMethod::Auto => unreachable!(),
        },
        may_discard: MayDiscard(matches!(material.alpha_mode(), AlphaMode::Mask(_))),
        material_data: prepared.data,
    };
    let packed_key = pipeline_cache.pack_key(&key);
    Ok(PreparedMaterial {
        bindings: prepared.bindings,
        bind_group: prepared.bind_group,
        new_packed_key: packed_key,
        properties: MaterialProperties {
            alpha_mode: material.alpha_mode(),
            depth_bias: material.depth_bias(),
            reads_view_transmission_texture: material.reads_view_transmission_texture(),
            render_method: method,
        },
    })
}

#[derive(PipelineKey, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
#[custom_shader_defs]
pub enum AlphaKey {
    Opaque,
    AlphaBlend,
    AlphaPremultiplied,
    AlphaMultiply,
    AlphaMask,
}

impl KeyShaderDefs for AlphaKey {
    fn shader_defs(&self) -> Vec<ShaderDefVal> {
        match self {
            AlphaKey::Opaque => vec![],
            AlphaKey::AlphaBlend => vec!["BLEND_ALPHA"],
            AlphaKey::AlphaPremultiplied => vec!["BLEND_PREMULTIPLIED_ALPHA", "PREMULTIPLY_ALPHA"],
            AlphaKey::AlphaMultiply => vec!["BLEND_MULTIPLY", "PREMULTIPLY_ALPHA"],
            AlphaKey::AlphaMask => vec!["MAY_DISCARD"],
        }
        .into_iter()
        .map(Into::into)
        .collect()
    }
}

impl From<AlphaMode> for AlphaKey {
    fn from(value: AlphaMode) -> Self {
        match value {
            AlphaMode::Opaque => AlphaKey::Opaque,
            AlphaMode::Mask(_) => AlphaKey::AlphaMask,
            AlphaMode::Blend => AlphaKey::AlphaBlend,
            AlphaMode::Premultiplied | AlphaMode::Add => AlphaKey::AlphaPremultiplied,
            AlphaMode::Multiply => AlphaKey::AlphaMultiply,
        }
    }
}

#[derive(PipelineKey, Clone, Copy)]
#[custom_shader_defs]
pub struct MayDiscard(pub bool);

impl KeyShaderDefs for MayDiscard {
    fn shader_defs(&self) -> Vec<ShaderDefVal> {
        if self.0 {
            vec!["MAY_DISCARD".into()]
        } else {
            vec![]
        }
    }
}

#[derive(PipelineKey, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum OpaqueMethodKey {
    Forward,
    Deferred,
}

#[derive(PipelineKey, Clone, Copy)]
pub struct NewMaterialKey<M: Material> {
    pub alpha: AlphaKey,
    pub opaque_method: OpaqueMethodKey,
    pub may_discard: MayDiscard,
    pub material_data: M::Data,
}

#[derive(PipelineKey, Clone, Copy)]
#[repr(u8)]
#[custom_shader_defs]
pub enum ScreenSpaceTransmissionQualityKey {
    None,
    Low,
    Medium,
    High,
    Ultra,
}

impl SystemKey for ScreenSpaceTransmissionQualityKey {
    type Param = ();

    type Query = Option<Read<Camera3d>>;

    fn from_params(_: &(), camera: Option<&Camera3d>) -> Option<Self>
    where
        Self: Sized,
    {
        let Some(camera) = camera else {
            return Some(ScreenSpaceTransmissionQualityKey::None);
        };
        Some(match camera.screen_space_specular_transmission_quality {
            ScreenSpaceTransmissionQuality::Low => Self::Low,
            ScreenSpaceTransmissionQuality::Medium => Self::Medium,
            ScreenSpaceTransmissionQuality::High => Self::High,
            ScreenSpaceTransmissionQuality::Ultra => Self::Ultra,
        })
    }
}

impl KeyShaderDefs for ScreenSpaceTransmissionQualityKey {
    fn shader_defs(&self) -> Vec<ShaderDefVal> {
        let taps = match self {
            ScreenSpaceTransmissionQualityKey::None => return Vec::default(),
            ScreenSpaceTransmissionQualityKey::Low => 4,
            ScreenSpaceTransmissionQualityKey::Medium => 8,
            ScreenSpaceTransmissionQualityKey::High => 16,
            ScreenSpaceTransmissionQualityKey::Ultra => 32,
        };
        vec![ShaderDefVal::UInt(
            "SCREEN_SPACE_SPECULAR_TRANSMISSION_BLUR_TAPS".into(),
            taps,
        )]
    }
}
