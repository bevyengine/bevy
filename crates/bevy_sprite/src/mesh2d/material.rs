use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, Asset, AssetServer, Handle};
use bevy_core_pipeline::core_2d::Transparent2d;
use bevy_ecs::{
    entity::Entity,
    prelude::{Bundle, World},
    system::{
        lifetimeless::{Read, SQuery, SRes},
        Query, Res, ResMut, SystemParamItem,
    },
    world::FromWorld,
};
use bevy_log::error;
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    mesh::{Mesh, MeshVertexBufferLayout},
    render_asset::{RenderAsset, RenderAssetPlugin, RenderAssets},
    render_phase::{
        AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
        SetItemPipeline, TrackedRenderPass,
    },
    render_resource::{
        BindGroup, BindGroupLayout, PipelineCache, RenderPipelineDescriptor, Shader,
        SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
    },
    renderer::RenderDevice,
    view::{ComputedVisibility, Msaa, Visibility, VisibleEntities},
    RenderApp, RenderStage,
};
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_utils::FloatOrd;
use std::hash::Hash;
use std::marker::PhantomData;

use crate::{
    DrawMesh2d, Mesh2dHandle, Mesh2dPipeline, Mesh2dPipelineKey, Mesh2dUniform, SetMesh2dBindGroup,
    SetMesh2dViewBindGroup,
};

/// Materials are used alongside [`Material2dPlugin`] and [`MaterialMesh2dBundle`]
/// to spawn entities that are rendered with a specific [`Material2d`] type. They serve as an easy to use high level
/// way to render [`Mesh2dHandle`] entities with custom shader logic. For materials that can specialize their [`RenderPipelineDescriptor`]
/// based on specific material values, see [`SpecializedMaterial2d`]. [`Material2d`] automatically implements [`SpecializedMaterial2d`]
/// and can be used anywhere that type is used (such as [`Material2dPlugin`]).
pub trait Material2d: Asset + RenderAsset {
    /// Returns this material's [`BindGroup`]. This should match the layout returned by [`Material2d::bind_group_layout`].
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup;

    /// Returns this material's [`BindGroupLayout`]. This should match the [`BindGroup`] returned by [`Material2d::bind_group`].
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

impl<M: Material2d> SpecializedMaterial2d for M {
    type Key = ();

    #[inline]
    fn key(
        _render_device: &RenderDevice,
        _material: &<Self as RenderAsset>::PreparedAsset,
    ) -> Self::Key {
    }

    #[inline]
    fn specialize(
        _key: Self::Key,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError> {
        <M as Material2d>::specialize(descriptor, layout)
    }

    #[inline]
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup {
        <M as Material2d>::bind_group(material)
    }

    #[inline]
    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        <M as Material2d>::bind_group_layout(render_device)
    }

    #[inline]
    fn vertex_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        <M as Material2d>::vertex_shader(asset_server)
    }

    #[inline]
    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        <M as Material2d>::fragment_shader(asset_server)
    }

    #[allow(unused_variables)]
    #[inline]
    fn dynamic_uniform_indices(material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        <M as Material2d>::dynamic_uniform_indices(material)
    }
}

/// Materials are used alongside [`Material2dPlugin`] and [`MaterialMesh2dBundle`](crate::MaterialMesh2dBundle)
/// to spawn entities that are rendered with a specific [`SpecializedMaterial2d`] type. They serve as an easy to use high level
/// way to render [`Mesh2dHandle`] entities with custom shader logic. [`SpecializedMaterial2d`s](SpecializedMaterial2d) use their [`SpecializedMaterial2d::Key`]
/// to customize their [`RenderPipelineDescriptor`] based on specific material values. The slightly simpler [`Material2d`] trait
/// should be used for materials that do not need specialization. [`Material2d`] types automatically implement [`SpecializedMaterial2d`].
pub trait SpecializedMaterial2d: Asset + RenderAsset {
    /// The key used to specialize this material's [`RenderPipelineDescriptor`].
    type Key: PartialEq + Eq + Hash + Clone + Send + Sync;

    /// Extract the [`SpecializedMaterial2d::Key`] for the "prepared" version of this material. This key will be
    /// passed in to the [`SpecializedMaterial2d::specialize`] function when compiling the [`RenderPipeline`](bevy_render::render_resource::RenderPipeline)
    /// for a given entity's material.
    fn key(
        render_device: &RenderDevice,
        material: &<Self as RenderAsset>::PreparedAsset,
    ) -> Self::Key;

    /// Specializes the given `descriptor` according to the given `key`.
    fn specialize(
        key: Self::Key,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayout,
    ) -> Result<(), SpecializedMeshPipelineError>;

    /// Returns this material's [`BindGroup`]. This should match the layout returned by [`SpecializedMaterial2d::bind_group_layout`].
    fn bind_group(material: &<Self as RenderAsset>::PreparedAsset) -> &BindGroup;

    /// Returns this material's [`BindGroupLayout`]. This should match the [`BindGroup`] returned by [`SpecializedMaterial2d::bind_group`].
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

    /// The dynamic uniform indices to set for the given `material`'s [`BindGroup`].
    /// Defaults to an empty array / no dynamic uniform indices.
    #[allow(unused_variables)]
    #[inline]
    fn dynamic_uniform_indices(material: &<Self as RenderAsset>::PreparedAsset) -> &[u32] {
        &[]
    }
}

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given [`SpecializedMaterial2d`]
/// asset type (which includes [`Material2d`] types).
pub struct Material2dPlugin<M: SpecializedMaterial2d>(PhantomData<M>);

impl<M: SpecializedMaterial2d> Default for Material2dPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: SpecializedMaterial2d> Plugin for Material2dPlugin<M> {
    fn build(&self, app: &mut App) {
        app.add_asset::<M>()
            .add_plugin(ExtractComponentPlugin::<Handle<M>>::extract_visible())
            .add_plugin(RenderAssetPlugin::<M>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent2d, DrawMaterial2d<M>>()
                .init_resource::<Material2dPipeline<M>>()
                .init_resource::<SpecializedMeshPipelines<Material2dPipeline<M>>>()
                .add_system_to_stage(RenderStage::Queue, queue_material2d_meshes::<M>);
        }
    }
}

pub struct Material2dPipeline<M: SpecializedMaterial2d> {
    pub mesh2d_pipeline: Mesh2dPipeline,
    pub material2d_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    marker: PhantomData<M>,
}

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct Material2dKey<T> {
    pub mesh_key: Mesh2dPipelineKey,
    pub material_key: T,
}

impl<M: SpecializedMaterial2d> SpecializedMeshPipeline for Material2dPipeline<M> {
    type Key = Material2dKey<M::Key>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh2d_pipeline.specialize(key.mesh_key, layout)?;
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }
        descriptor.layout = Some(vec![
            self.mesh2d_pipeline.view_layout.clone(),
            self.material2d_layout.clone(),
            self.mesh2d_pipeline.mesh_layout.clone(),
        ]);

        M::specialize(key.material_key, &mut descriptor, layout)?;
        Ok(descriptor)
    }
}

impl<M: SpecializedMaterial2d> FromWorld for Material2dPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let render_device = world.resource::<RenderDevice>();
        let material2d_layout = M::bind_group_layout(render_device);

        Material2dPipeline {
            mesh2d_pipeline: world.resource::<Mesh2dPipeline>().clone(),
            material2d_layout,
            vertex_shader: M::vertex_shader(asset_server),
            fragment_shader: M::fragment_shader(asset_server),
            marker: PhantomData,
        }
    }
}

type DrawMaterial2d<M> = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetMaterial2dBindGroup<M, 1>,
    SetMesh2dBindGroup<2>,
    DrawMesh2d,
);

pub struct SetMaterial2dBindGroup<M: SpecializedMaterial2d, const I: usize>(PhantomData<M>);
impl<M: SpecializedMaterial2d, const I: usize> EntityRenderCommand
    for SetMaterial2dBindGroup<M, I>
{
    type Param = (SRes<RenderAssets<M>>, SQuery<Read<Handle<M>>>);
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (materials, query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material2d_handle = query.get(item).unwrap();
        let material2d = materials.into_inner().get(material2d_handle).unwrap();
        pass.set_bind_group(
            I,
            M::bind_group(material2d),
            M::dynamic_uniform_indices(material2d),
        );
        RenderCommandResult::Success
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_material2d_meshes<M: SpecializedMaterial2d>(
    transparent_draw_functions: Res<DrawFunctions<Transparent2d>>,
    material2d_pipeline: Res<Material2dPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<Material2dPipeline<M>>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    render_device: Res<RenderDevice>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderAssets<M>>,
    material2d_meshes: Query<(&Handle<M>, &Mesh2dHandle, &Mesh2dUniform)>,
    mut views: Query<(&VisibleEntities, &mut RenderPhase<Transparent2d>)>,
) {
    if material2d_meshes.is_empty() {
        return;
    }
    let render_device = render_device.into_inner();
    for (visible_entities, mut transparent_phase) in views.iter_mut() {
        let draw_transparent_pbr = transparent_draw_functions
            .read()
            .get_id::<DrawMaterial2d<M>>()
            .unwrap();

        let msaa_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples);

        for visible_entity in &visible_entities.entities {
            if let Ok((material2d_handle, mesh2d_handle, mesh2d_uniform)) =
                material2d_meshes.get(*visible_entity)
            {
                if let Some(material2d) = render_materials.get(material2d_handle) {
                    if let Some(mesh) = render_meshes.get(&mesh2d_handle.0) {
                        let mesh_key = msaa_key
                            | Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology);

                        let material_key = M::key(render_device, material2d);
                        let pipeline_id = pipelines.specialize(
                            &mut pipeline_cache,
                            &material2d_pipeline,
                            Material2dKey {
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

                        let mesh_z = mesh2d_uniform.transform.w_axis.z;
                        transparent_phase.add(Transparent2d {
                            entity: *visible_entity,
                            draw_function: draw_transparent_pbr,
                            pipeline: pipeline_id,
                            // NOTE: Back-to-front ordering for transparent with ascending sort means far should have the
                            // lowest sort key and getting closer should increase. As we have
                            // -z in front of the camera, the largest distance is -far with values increasing toward the
                            // camera. As such we can just use mesh_z as the distance
                            sort_key: FloatOrd(mesh_z),
                            // This material is not batched
                            batch_range: None,
                        });
                    }
                }
            }
        }
    }
}

/// A component bundle for entities with a [`Mesh2dHandle`] and a [`SpecializedMaterial2d`].
#[derive(Bundle, Clone)]
pub struct MaterialMesh2dBundle<M: SpecializedMaterial2d> {
    pub mesh: Mesh2dHandle,
    pub material: Handle<M>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

impl<M: SpecializedMaterial2d> Default for MaterialMesh2dBundle<M> {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
        }
    }
}
