use crate::{
    AlphaMode, DrawMesh, MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup,
    SetMeshViewBindGroup, SetSkinnedMeshBindGroup,
};
use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, Asset, AssetServer, Handle};
use bevy_core_pipeline::{AlphaMask3d, Opaque3d, Transparent3d};
use bevy_ecs::{
    entity::Entity,
    prelude::World,
    system::{
        lifetimeless::{Read, SQuery, SRes},
        Query, Res, ResMut, SystemParamItem,
    },
    world::FromWorld,
};
use bevy_render::{
    mesh::{Mesh, MeshVertexBufferLayout},
    render_asset::{RenderAsset, RenderAssetPlugin, RenderAssets},
    render_component::ExtractComponentPlugin,
    render_phase::{
        AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
        SetItemPipeline, TrackedRenderPass,
    },
    render_resource::{
        BindGroup, BindGroupLayout, RenderPipelineCache, RenderPipelineDescriptor, Shader,
        SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
    },
    renderer::RenderDevice,
    view::{ExtractedView, Msaa, VisibleEntities},
    RenderApp, RenderStage,
};
use bevy_utils::tracing::error;
use std::hash::Hash;
use std::marker::PhantomData;

/// Materials are used alongside [`MaterialPlugin`] and [`MaterialMeshBundle`](crate::MaterialMeshBundle)
/// to spawn entities that are rendered with a specific [`Material`] type. They serve as an easy to use high level
/// way to render [`Mesh`] entities with custom shader logic. For materials that can specialize their [`RenderPipelineDescriptor`]
/// based on specific material values, see [`SpecializedMaterial`]. [`Material`] automatically implements [`SpecializedMaterial`]
/// and can be used anywhere that type is used (such as [`MaterialPlugin`]).
pub trait Material: Asset + RenderAsset {
    /// Returns this material's [`BindGroup`]. This should match the layout returned by [`Material::bind_group_layout`].
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup;

    /// Returns this material's [`BindGroupLayout`]. This should match the [`BindGroup`] returned by [`Material::bind_group`].
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout;

    /// Returns this material's vertex shader. If [`None`] is returned, the default mesh vertex shader will be used.
    /// Defaults to [`None`].
    #[allow(unused_variables)]
    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        None
    }

    /// Returns this material's fragment shader. If [`None`] is returned, the default mesh fragment shader will be used.
    /// Defaults to [`None`].
    #[allow(unused_variables)]
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        None
    }

    /// Returns this material's [`AlphaMode`]. Defaults to [`AlphaMode::Opaque`].
    #[allow(unused_variables)]
    fn alpha_mode(material: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        AlphaMode::Opaque
    }

    /// The dynamic uniform indices to set for the given `material`'s [`BindGroup`].
    /// Defaults to an empty array / no dynamic uniform indices.
    #[allow(unused_variables)]
    #[inline]
    fn dynamic_uniform_indices(material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        &[]
    }

    /// Customizes the default [`RenderPipelineDescriptor`].
    #[allow(unused_variables)]
    #[inline]
    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Ok(())
    }
}

impl<M: Material> SpecializedMaterial for M {
    type Key = ();

    #[inline]
    fn key(_material: &<Self as RenderAsset>::PreparedAsset) -> Self::Key {}

    #[inline]
    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        <M as Material>::specialize(descriptor, layout)
    }

    #[inline]
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        <M as Material>::bind_group(material)
    }

    #[inline]
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        <M as Material>::bind_group_layout(render_device)
    }

    #[inline]
    fn alpha_mode(material: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        <M as Material>::alpha_mode(material)
    }

    #[inline]
    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        <M as Material>::vertex_shader(asset_server)
    }

    #[inline]
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        <M as Material>::fragment_shader(asset_server)
    }

    #[allow(unused_variables)]
    #[inline]
    fn dynamic_uniform_indices(material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        <M as Material>::dynamic_uniform_indices(material)
    }
}

/// Materials are used alongside [`MaterialPlugin`] and [`MaterialMeshBundle`](crate::MaterialMeshBundle)
/// to spawn entities that are rendered with a specific [`SpecializedMaterial`] type. They serve as an easy to use high level
/// way to render [`Mesh`] entities with custom shader logic. [`SpecializedMaterials`](SpecializedMaterial) use their [`SpecializedMaterial::Key`]
/// to customize their [`RenderPipelineDescriptor`] based on specific material values. The slightly simpler [`Material`] trait
/// should be used for materials that do not need specialization. [`Material`] types automatically implement [`SpecializedMaterial`].
pub trait SpecializedMaterial: Asset + RenderAsset {
    /// The key used to specialize this material's [`RenderPipelineDescriptor`].
    type Key: PartialEq + Eq + Hash + Clone + Send + Sync;

    /// Extract the [`SpecializedMaterial::Key`] for the "prepared" version of this material. This key will be
    /// passed in to the [`SpecializedMaterial::specialize`] function when compiling the [`RenderPipeline`](bevy_render::render_resource::RenderPipeline)
    /// for a given entity's material.
    fn key(material: &<Self as RenderAsset>::PreparedAsset) -> Self::Key;

    /// Specializes the given `descriptor` according to the given `key`.
    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError>;

    /// Returns this material's [`BindGroup`]. This should match the layout returned by [`SpecializedMaterial::bind_group_layout`].
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup;

    /// Returns this material's [`BindGroupLayout`]. This should match the [`BindGroup`] returned by [`SpecializedMaterial::bind_group`].
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout;

    /// Returns this material's vertex shader. If [`None`] is returned, the default mesh vertex shader will be used.
    /// Defaults to [`None`].
    #[allow(unused_variables)]
    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        None
    }

    /// Returns this material's fragment shader. If [`None`] is returned, the default mesh fragment shader will be used.
    /// Defaults to [`None`].
    #[allow(unused_variables)]
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        None
    }

    /// Returns this material's [`AlphaMode`]. Defaults to [`AlphaMode::Opaque`].
    #[allow(unused_variables)]
    fn alpha_mode(material: &<Self as RenderAsset>::PreparedAsset) -> AlphaMode {
        AlphaMode::Opaque
    }

    /// The dynamic uniform indices to set for the given `material`'s [`BindGroup`].
    /// Defaults to an empty array / no dynamic uniform indices.
    #[allow(unused_variables)]
    #[inline]
    fn dynamic_uniform_indices(material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        &[]
    }
}

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given [`SpecializedMaterial`]
/// asset type (which includes [`Material`] types).
pub struct MaterialPlugin<M: SpecializedMaterial>(PhantomData<M>);

impl<M: SpecializedMaterial> Default for MaterialPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: SpecializedMaterial> Plugin for MaterialPlugin<M> {
    fn build(&self, app: &mut App) {
        app.add_asset::<M>()
            .add_plugin(ExtractComponentPlugin::<Handle<M>>::default())
            .add_plugin(RenderAssetPlugin::<M>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent3d, DrawMaterial<M>>()
                .add_render_command::<Opaque3d, DrawMaterial<M>>()
                .add_render_command::<AlphaMask3d, DrawMaterial<M>>()
                .init_resource::<MaterialPipeline<M>>()
                .init_resource::<SpecializedMeshPipelines<MaterialPipeline<M>>>()
                .add_system_to_stage(RenderStage::Queue, queue_material_meshes::<M>);
        }
    }
}

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct MaterialPipelineKey<T> {
    mesh_key: MeshPipelineKey,
    material_key: T,
}

pub struct MaterialPipeline<M: SpecializedMaterial> {
    pub mesh_pipeline: MeshPipeline,
    pub material_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    marker: PhantomData<M>,
}

impl<M: SpecializedMaterial> SpecializedMeshPipeline for MaterialPipeline<M> {
    type Key = MaterialPipelineKey<M::Key>;

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

        let descriptor_layout = descriptor.layout.as_mut().unwrap();
        descriptor_layout.insert(1, self.material_layout.clone());

        M::specialize(&mut descriptor, key.material_key, layout)?;
        Ok(descriptor)
    }
}

impl<M: SpecializedMaterial> FromWorld for MaterialPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let render_device = world.resource::<RenderDevice>();
        let material_layout = M::bind_group_layout(render_device);

        MaterialPipeline {
            mesh_pipeline: world.resource::<MeshPipeline>().clone(),
            material_layout,
            vertex_shader: M::vertex_shader(asset_server),
            fragment_shader: M::fragment_shader(asset_server),
            marker: PhantomData,
        }
    }
}

type DrawMaterial<M> = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<M, 1>,
    SetMeshBindGroup<2>,
    SetSkinnedMeshBindGroup<3>,
    DrawMesh,
);

pub struct SetMaterialBindGroup<M: SpecializedMaterial, const I: usize>(PhantomData<M>);
impl<M: SpecializedMaterial, const I: usize> EntityRenderCommand for SetMaterialBindGroup<M, I> {
    type Param = (SRes<RenderAssets<M>>, SQuery<Read<Handle<M>>>);
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (materials, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material_handle = query.get(item).unwrap();
        let material = materials.into_inner().get(material_handle).unwrap();
        pass.set_bind_group(
            I,
            M::bind_group(material),
            M::dynamic_uniform_indices(material),
        );
        RenderCommandResult::Success
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_material_meshes<M: SpecializedMaterial>(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    material_pipeline: Res<MaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<MaterialPipeline<M>>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderAssets<M>>,
    material_meshes: Query<(&Handle<M>, &Handle<Mesh>, &MeshUniform)>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        &mut RenderPhase<Opaque3d>,
        &mut RenderPhase<AlphaMask3d>,
        &mut RenderPhase<Transparent3d>,
    )>,
) {
    for (view, visible_entities, mut opaque_phase, mut alpha_mask_phase, mut transparent_phase) in
        views.iter_mut()
    {
        let draw_opaque_pbr = opaque_draw_functions
            .read()
            .get_id::<DrawMaterial<M>>()
            .unwrap();
        let draw_alpha_mask_pbr = alpha_mask_draw_functions
            .read()
            .get_id::<DrawMaterial<M>>()
            .unwrap();
        let draw_transparent_pbr = transparent_draw_functions
            .read()
            .get_id::<DrawMaterial<M>>()
            .unwrap();

        let inverse_view_matrix = view.transform.compute_matrix().inverse();
        let inverse_view_row_2 = inverse_view_matrix.row(2);
        let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

        for visible_entity in &visible_entities.entities {
            if let Ok((material_handle, mesh_handle, mesh_uniform)) =
                material_meshes.get(*visible_entity)
            {
                if let Some(material) = render_materials.get(material_handle) {
                    if let Some(mesh) = render_meshes.get(mesh_handle) {
                        let mut mesh_key =
                            MeshPipelineKey::from_primitive_topology(mesh.primitive_topology)
                                | msaa_key;
                        let alpha_mode = M::alpha_mode(material);
                        if let AlphaMode::Blend = alpha_mode {
                            mesh_key |= MeshPipelineKey::TRANSPARENT_MAIN_PASS;
                        }

                        let material_key = M::key(material);

                        let pipeline_id = pipelines.specialize(
                            &mut pipeline_cache,
                            &material_pipeline,
                            MaterialPipelineKey {
                                mesh_key,
                                material_key,
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

                        // NOTE: row 2 of the inverse view matrix dotted with column 3 of the model matrix
                        // gives the z component of translation of the mesh in view space
                        let mesh_z = inverse_view_row_2.dot(mesh_uniform.transform.col(3));
                        match alpha_mode {
                            AlphaMode::Opaque => {
                                opaque_phase.add(Opaque3d {
                                    entity: *visible_entity,
                                    draw_function: draw_opaque_pbr,
                                    pipeline: pipeline_id,
                                    // NOTE: Front-to-back ordering for opaque with ascending sort means near should have the
                                    // lowest sort key and getting further away should increase. As we have
                                    // -z in front of the camera, values in view space decrease away from the
                                    // camera. Flipping the sign of mesh_z results in the correct front-to-back ordering
                                    distance: -mesh_z,
                                });
                            }
                            AlphaMode::Mask(_) => {
                                alpha_mask_phase.add(AlphaMask3d {
                                    entity: *visible_entity,
                                    draw_function: draw_alpha_mask_pbr,
                                    pipeline: pipeline_id,
                                    // NOTE: Front-to-back ordering for alpha mask with ascending sort means near should have the
                                    // lowest sort key and getting further away should increase. As we have
                                    // -z in front of the camera, values in view space decrease away from the
                                    // camera. Flipping the sign of mesh_z results in the correct front-to-back ordering
                                    distance: -mesh_z,
                                });
                            }
                            AlphaMode::Blend => {
                                transparent_phase.add(Transparent3d {
                                    entity: *visible_entity,
                                    draw_function: draw_transparent_pbr,
                                    pipeline: pipeline_id,
                                    // NOTE: Back-to-front ordering for transparent with ascending sort means far should have the
                                    // lowest sort key and getting closer should increase. As we have
                                    // -z in front of the camera, the largest distance is -far with values increasing toward the
                                    // camera. As such we can just use mesh_z as the distance
                                    distance: mesh_z,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}
